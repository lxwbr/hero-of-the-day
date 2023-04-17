import { Stack, StackProps, CfnOutput, Duration } from 'aws-cdk-lib';
import { Construct } from 'constructs';
import { RustFunction } from 'rust.aws-cdk-lambda';
import * as dynamodb from 'aws-cdk-lib/aws-dynamodb';
import * as apigw from 'aws-cdk-lib/aws-apigateway';
import { AttributeType, BillingMode, ITable, Table } from 'aws-cdk-lib/aws-dynamodb';
import { IFunction } from 'aws-cdk-lib/aws-lambda';
import { IRule, Rule, Schedule } from 'aws-cdk-lib/aws-events';
import { LambdaFunction } from 'aws-cdk-lib/aws-events-targets';
import { IParameter, StringParameter } from 'aws-cdk-lib/aws-ssm';

const heroOfTheDay = 'hero-of-the-day'
const environment = {
  "HERO_TABLE": `${heroOfTheDay}-hero`,
  "USER_TABLE": `${heroOfTheDay}-user`,
  "PUNCH_CLOCK_TABLE": `${heroOfTheDay}-punch-clock`,
  "SCHEDULE_TABLE": `${heroOfTheDay}-schedule`,
  "OLD_HERO_TABLE": 'hero-of-the-day-dev-hero',
  "OLD_SCHEDULE_TABLE": 'hero-of-the-day-dev-schedule',
  "OLD_USER_TABLE": 'hero-of-the-day-dev-user',
  "HOSTED_DOMAIN": 'xxx',
  "GOOGLE_CLIENT_ID": 'xxx',
  "MS_CLIENT_ID": 'xxx',
  "SLACK_TOKEN_PARAMETER": `/${heroOfTheDay}/slack-token`
}

export class HeroOfTheDayStack extends Stack {
  constructor(scope: Construct, id: string, props?: StackProps) {
    super(scope, id, props);

    let heroTable: ITable = this.heroTable();
    let userTable: ITable = this.userTable();
    let scheduleTable: ITable = this.scheduleTable();
    let punchClockTable: ITable = this.punchClockTable();

    let slackParameter = StringParameter.fromStringParameterName(this, 'SlackParameter', environment.SLACK_TOKEN_PARAMETER);

    let authorizer: IFunction = this.authorizer(heroTable, userTable);
    let heroListFn: IFunction = this.heroList(heroTable);
    let heroGetFn: IFunction = this.heroGet(heroTable);
    let heroPutFn: IFunction = this.heroPut(heroTable);
    let userCreateFn: IFunction = this.userCreate(userTable);
    let scheduleGetFn: IFunction = this.scheduleGet(scheduleTable);
    let scheduleUpdateFn: IFunction = this.scheduleUpdate(scheduleTable, heroTable, slackParameter);
    let slackUsergroupUsersUpdateFn: IFunction = this.slackUsergroupUsersUpdate(scheduleTable, heroTable, punchClockTable, slackParameter);
    let heroMemberDeleteFn: IFunction = this.heroMemeberDelete(heroTable);
    let heroDeleteFn: IFunction = this.heroDelete(heroTable, scheduleTable);
    let punchClockRecalculateFn: IFunction = this.punchClockRecalculate(scheduleTable, punchClockTable, slackParameter);

    this.slackUsergroupUsersUpdateScheduleRule(slackUsergroupUsersUpdateFn);

    this.migrate(heroTable, userTable, scheduleTable);

    this.apiGateway(authorizer, heroListFn, heroGetFn, userCreateFn, scheduleGetFn, scheduleUpdateFn, heroPutFn, heroMemberDeleteFn, heroDeleteFn, punchClockRecalculateFn);
  }

  slackUsergroupUsersUpdateScheduleRule(slackUsergroupUsersUpdateFn: IFunction): IRule {
    let rule = new Rule(this, 'SlackUsergroupUsersUpdateScheduleRule', {
      schedule: Schedule.cron({ minute: '0', hour: '0' }),
      targets: [new LambdaFunction(slackUsergroupUsersUpdateFn)],
     });

    return rule;
  }

  heroTable(): ITable {
    let table = new dynamodb.Table(this, environment.HERO_TABLE, {
      tableName: environment.HERO_TABLE,
      partitionKey: {
        name: 'name',
        type: AttributeType.STRING
      },
      billingMode: BillingMode.PAY_PER_REQUEST
    });
    return table;
  }

  userTable(): ITable {
    let table = new dynamodb.Table(this, environment.USER_TABLE, {
      tableName: environment.USER_TABLE,
      partitionKey: {
        name: 'email',
        type: AttributeType.STRING
      },
      billingMode: BillingMode.PAY_PER_REQUEST
    });
    return table;
  }

  punchClockTable(): ITable {
    let table = new dynamodb.Table(this, environment.PUNCH_CLOCK_TABLE, {
      tableName: environment.PUNCH_CLOCK_TABLE,
      partitionKey: {
        name: 'hero',
        type: AttributeType.STRING
      },
      sortKey: {
        name: 'member',
        type: AttributeType.STRING
      },
      billingMode: BillingMode.PAY_PER_REQUEST
    });
    return table;
  }

  scheduleTable(): ITable {
    let table = new dynamodb.Table(this, environment.SCHEDULE_TABLE, {
      tableName: environment.SCHEDULE_TABLE,
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
    return table;
  }

  createFn(id: string, name: string, timeout: Duration = Duration.seconds(3)): IFunction {
    return new RustFunction(this, id, {
      package: name,
      environment,
      functionName: `${heroOfTheDay}-${name}`,
      timeout
    });
  }

  migrate(heroTable: ITable, userTable: ITable, scheduleTable: ITable): IFunction {
    let oldHeroTable = Table.fromTableArn(this, 'OldHeroTable', `arn:aws:dynamodb:eu-central-1:514130831484:table/${environment.OLD_HERO_TABLE}`);
    let oldUserTable = Table.fromTableArn(this, 'OldUserTable', `arn:aws:dynamodb:eu-central-1:514130831484:table/${environment.OLD_USER_TABLE}`);
    let oldScheduleTable = Table.fromTableArn(this, 'OldScheduleTable', `arn:aws:dynamodb:eu-central-1:514130831484:table/${environment.OLD_SCHEDULE_TABLE}`);

    let fn = this.createFn('MigrateFunction', 'migrate', Duration.seconds(50));
    heroTable.grantReadWriteData(fn);
    userTable.grantReadWriteData(fn);
    scheduleTable.grantReadWriteData(fn);
    oldHeroTable.grantReadData(fn);
    oldUserTable.grantReadData(fn);
    oldScheduleTable.grantReadData(fn);
    return fn;
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

  slackUsergroupUsersUpdate(scheduleTable: ITable, heroTable: ITable, punchClockTable: ITable, slackParameter: IParameter): IFunction {
    let fn = this.createFn('SlackUsergroupUsersUpdateFunction', 'slack-usergroup-users-update', Duration.seconds(50));
    scheduleTable.grantReadData(fn);
    heroTable.grantReadData(fn);
    slackParameter.grantRead(fn);
    punchClockTable.grantReadWriteData(fn);
    return fn;
  }

  punchClockRecalculate(scheduleTable: ITable, punchClockTable: ITable, slackParameter: IParameter): IFunction {
    let fn = this.createFn('PunchClockRecalculateFunction', 'punch-clock-recalculate');
    scheduleTable.grantReadData(fn);
    slackParameter.grantRead(fn);
    punchClockTable.grantReadWriteData(fn);
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
    heroDeleteFn: IFunction,
    punchClockRecalculateFn: IFunction
  ) {
    const api = new apigw.RestApi(this, `${heroOfTheDay}-api`, {
      description: heroOfTheDay,
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
    let punchclockPath = api.root.addResource('punchclock');
    let recalculatePath = punchclockPath.addResource('recalculate')

    let authorizer = new apigw.TokenAuthorizer(this, 'HeroOfTheDayCustomAuthorizer', {
      handler: authorizerFn,
      resultsCacheTtl: Duration.minutes(0),
      authorizerName: `${heroOfTheDay}-authorizer`
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

    const punchclockRecalculateResource = recalculatePath.addResource('{hero}');

    punchclockRecalculateResource.addMethod('POST', new apigw.LambdaIntegration(punchClockRecalculateFn, { proxy: true }),
      {
        authorizer,
        authorizationType: apigw.AuthorizationType.CUSTOM
      }
    )
  }
}
