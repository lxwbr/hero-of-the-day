import {CfnOutput, Duration, Stack, StackProps} from 'aws-cdk-lib';
import {Construct} from 'constructs';
import {RustFunction} from 'cargo-lambda-cdk';
import * as dynamodb from 'aws-cdk-lib/aws-dynamodb';
import {AttributeType, BillingMode, ITable} from 'aws-cdk-lib/aws-dynamodb';
import * as apigw from 'aws-cdk-lib/aws-apigateway';
import {IFunction} from 'aws-cdk-lib/aws-lambda';
import {IRule, Rule, Schedule} from 'aws-cdk-lib/aws-events';
import {LambdaFunction} from 'aws-cdk-lib/aws-events-targets';
import {IParameter, StringParameter} from 'aws-cdk-lib/aws-ssm';

interface Environment {
  readonly APP_NAME: string;
  readonly HERO_TABLE: string,
  readonly USER_TABLE: string,
  readonly SCHEDULE_TABLE: string,
  readonly HOSTED_DOMAIN: string,
  readonly MS_CLIENT_ID: string,
  readonly SLACK_TOKEN_PARAMETER: string
}

export class HeroOfTheDayStack extends Stack {
  env: Environment;
  constructor(scope: Construct, id: string, props: StackProps & Environment) {
    super(scope, id, props);
    this.env = props;

    let heroTable: ITable = this.heroTable();
    let userTable: ITable = this.userTable();
    let scheduleTable: ITable = this.scheduleTable();

    let slackParameter = StringParameter.fromStringParameterName(this, 'SlackParameter', this.env.SLACK_TOKEN_PARAMETER);

    let authorizer: IFunction = this.authorizer(heroTable, userTable);
    let heroListFn: IFunction = this.heroList(heroTable);
    let heroGetFn: IFunction = this.heroGet(heroTable);
    let heroPutFn: IFunction = this.heroPut(heroTable);
    let userCreateFn: IFunction = this.userCreate(userTable);
    let scheduleGetFn: IFunction = this.scheduleGet(scheduleTable);
    let scheduleUpdateFn: IFunction = this.scheduleUpdate(scheduleTable, heroTable, slackParameter);
    let slackUsergroupUsersUpdateFn: IFunction = this.slackUsergroupUsersUpdate(scheduleTable, heroTable, slackParameter);
    let heroMemberDeleteFn: IFunction = this.heroMemeberDelete(heroTable);
    let heroDeleteFn: IFunction = this.heroDelete(heroTable, scheduleTable);

    this.slackUsergroupUsersUpdateScheduleRule(slackUsergroupUsersUpdateFn);

    this.apiGateway(authorizer, heroListFn, heroGetFn, userCreateFn, scheduleGetFn, scheduleUpdateFn, heroPutFn, heroMemberDeleteFn, heroDeleteFn);
  }

  slackUsergroupUsersUpdateScheduleRule(slackUsergroupUsersUpdateFn: IFunction): IRule {
    return new Rule(this, 'SlackUsergroupUsersUpdateScheduleRule', {
      schedule: Schedule.cron({minute: '0', hour: '0'}),
      targets: [new LambdaFunction(slackUsergroupUsersUpdateFn)],
    });
  }

  heroTable(): ITable {
    return new dynamodb.Table(this, this.env.HERO_TABLE, {
      tableName: this.env.HERO_TABLE,
      partitionKey: {
        name: 'name',
        type: AttributeType.STRING
      },
      billingMode: BillingMode.PAY_PER_REQUEST
    });
  }

  userTable(): ITable {
    return new dynamodb.Table(this, this.env.USER_TABLE, {
      tableName: this.env.USER_TABLE,
      partitionKey: {
        name: 'email',
        type: AttributeType.STRING
      },
      billingMode: BillingMode.PAY_PER_REQUEST
    });
  }

  scheduleTable(): ITable {
    return new dynamodb.Table(this, this.env.SCHEDULE_TABLE, {
      tableName: this.env.SCHEDULE_TABLE,
      partitionKey: {
        name: 'hero',
        type: AttributeType.STRING
      },
      sortKey: {
        name: 'shift_start_time',
        type: AttributeType.NUMBER
      },
      billingMode: BillingMode.PAY_PER_REQUEST
    });
  }

  createFn(id: string, name: string, timeout: Duration = Duration.seconds(3)): IFunction {
    return new RustFunction(this, id, {
      manifestPath: `../${name}/Cargo.toml`,
      functionName: `${this.env.APP_NAME}-${name}`,
      timeout,
      environment: {
        APP_NAME: this.env.APP_NAME,
        HERO_TABLE: this.env.HERO_TABLE,
        USER_TABLE: this.env.USER_TABLE,
        SCHEDULE_TABLE: this.env.SCHEDULE_TABLE,
        HOSTED_DOMAIN: this.env.HOSTED_DOMAIN,
        MS_CLIENT_ID: this.env.MS_CLIENT_ID,
        SLACK_TOKEN_PARAMETER: this.env.SLACK_TOKEN_PARAMETER
      }
    });
  }

  authorizer(heroTable: ITable, userTable: ITable): IFunction {
    let fn = this.createFn('AuthorizerFunction', 'authorizer');
    heroTable.grantReadData(fn);
    userTable.grantReadWriteData(fn);
    return fn;
  }

  heroList(table: ITable): IFunction {
    let fn = this.createFn('HeroListFunction', 'hero-list');
    table.grantReadData(fn);
    return fn;
  }

  heroGet(table: ITable): IFunction {
    let fn = this.createFn('HeroGetFunction', 'hero-get');
    table.grantReadData(fn);
    return fn;
  }

  heroPut(table: ITable): IFunction {
    let fn = this.createFn('HeroCreateFunction', 'hero-put');
    table.grantReadWriteData(fn);
    return fn;
  }

  userCreate(table: ITable): IFunction {
    let fn = this.createFn('UserCreateFunction', 'user-create');
    table.grantReadWriteData(fn);
    return fn;
  }

  scheduleGet(table: ITable): IFunction {
    let fn = this.createFn('ScheduleGetFunction', 'schedule-get');
    table.grantReadData(fn);
    return fn;
  } 

  scheduleUpdate(scheduleTable: ITable, heroTable: ITable, slackParameter: IParameter): IFunction {
    let fn = this.createFn('ScheduleUpdateFunction', 'schedule-update');
    scheduleTable.grantReadWriteData(fn);
    heroTable.grantReadWriteData(fn);
    slackParameter.grantRead(fn);
    return fn;
  }

  slackUsergroupUsersUpdate(scheduleTable: ITable, heroTable: ITable, slackParameter: IParameter): IFunction {
    let fn = this.createFn('SlackUsergroupUsersUpdateFunction', 'slack-usergroup-users-update', Duration.seconds(50));
    scheduleTable.grantReadData(fn);
    heroTable.grantReadData(fn);
    slackParameter.grantRead(fn);
    return fn;
  }

  heroMemeberDelete(heroTable: ITable): IFunction {
    let fn = this.createFn('HeroMemberDelete', 'hero-delete-member');
    heroTable.grantReadWriteData(fn);
    return fn;
  }

  heroDelete(heroTable: ITable, scheduleTable: ITable): IFunction {
    let fn = this.createFn('HeroDelete', 'hero-delete');
    heroTable.grantReadWriteData(fn);
    scheduleTable.grantReadWriteData(fn);
    return fn;
  }

  apiGateway(
    authorizerFn: IFunction,
    heroListFn: IFunction,
    heroGetFn: IFunction,
    userCreateFn: IFunction,
    scheduleGetFn: IFunction,
    scheduleUpdateFn: IFunction,
    heroPutFn: IFunction,
    heroMemberDeleteFn: IFunction,
    heroDeleteFn: IFunction
  ) {
    const api = new apigw.RestApi(this, `${this.env.APP_NAME}-api`, {
      description: this.env.APP_NAME,
      defaultCorsPreflightOptions: {
        statusCode: 200,
        allowHeaders: [
          'Content-Type','X-Amz-Date','Authorization','X-Api-Key','X-Amz-Security-Token','X-Amz-User-Agent'
        ],
        allowOrigins: apigw.Cors.ALL_ORIGINS,
        allowMethods: ["POST", "PUT", "GET", "DELETE", "OPTIONS"],
      }
    });

    new CfnOutput(this, 'apiUrl', { value: api.url });

    let heroPath = api.root.addResource('hero');
    let userPath = api.root.addResource('user');
    let schedulePath = api.root.addResource('schedule');

    let authorizer = new apigw.TokenAuthorizer(this, 'HeroOfTheDayCustomAuthorizer', {
      handler: authorizerFn,
      resultsCacheTtl: Duration.minutes(0),
      authorizerName: `${this.env.APP_NAME}-authorizer`
    })

    heroPath.addResource('list').addMethod('GET',
      new apigw.LambdaIntegration(heroListFn, { proxy: true }),
      {
        authorizer,
        authorizationType: apigw.AuthorizationType.CUSTOM
      }
    );

    let heroHeroPathResource = heroPath.addResource('{hero}');
    heroHeroPathResource.addMethod('GET',
      new apigw.LambdaIntegration(heroGetFn, { proxy: true }), 
        {
          authorizer,
          authorizationType: apigw.AuthorizationType.CUSTOM
        }
    )

    heroHeroPathResource.addMethod('PUT',
      new apigw.LambdaIntegration(heroPutFn, { proxy: true }), 
        {
          authorizer,
          authorizationType: apigw.AuthorizationType.CUSTOM
        }
    )

    heroHeroPathResource.addMethod('DELETE',
      new apigw.LambdaIntegration(heroDeleteFn, { proxy: true }), 
      {
        authorizer,
        authorizationType: apigw.AuthorizationType.CUSTOM
      }
    )

    heroHeroPathResource.addResource('members').addResource('{member}').addMethod('DELETE',
      new apigw.LambdaIntegration(heroMemberDeleteFn, { proxy: true }), 
      {
        authorizer,
        authorizationType: apigw.AuthorizationType.CUSTOM
      }
    )

    userPath.addResource('{user}').addMethod('PUT',
      new apigw.LambdaIntegration(userCreateFn, { proxy: true }), 
      {
        authorizer,
        authorizationType: apigw.AuthorizationType.CUSTOM
      }
    )

    const heroResource = schedulePath.addResource('{hero}');

    heroResource.addMethod('GET', new apigw.LambdaIntegration(scheduleGetFn, { proxy: true }), 
      {
        authorizer,
        authorizationType: apigw.AuthorizationType.CUSTOM
      }
    )

    heroResource.addMethod('POST', new apigw.LambdaIntegration(scheduleUpdateFn, { proxy: true }), 
      {
        authorizer,
        authorizationType: apigw.AuthorizationType.CUSTOM
      }
    )
  }
}
