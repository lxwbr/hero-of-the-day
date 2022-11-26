import { Stack, StackProps, CfnOutput, Duration } from 'aws-cdk-lib';
import { Construct } from 'constructs';
import { RustFunction } from 'rust.aws-cdk-lambda';
import * as dynamodb from 'aws-cdk-lib/aws-dynamodb';
import * as apigw from 'aws-cdk-lib/aws-apigateway';
import { AttributeType, BillingMode, ITable, Table } from 'aws-cdk-lib/aws-dynamodb';
import { IFunction } from 'aws-cdk-lib/aws-lambda';

const heroOfTheDay = 'hero-of-the-day'
const environment = {
  "HERO_TABLE": `${heroOfTheDay}-hero`,
  "USER_TABLE": `${heroOfTheDay}-user`,
  "OLD_HERO_TABLE": 'hero-of-the-day-dev-hero',
  "OLD_USER_TABLE": 'hero-of-the-day-dev-user',
  "HOSTED_DOMAIN": 'moia.io',
  "GOOGLE_CLIENT_ID": '356839337273-s4slmoo3nc0odhjsocbqs0fjra3lvn3v.apps.googleusercontent.com',
  "MS_CLIENT_ID": 'aec83053-579c-4646-812e-cfa7d97ad9a0',
  "SLACK_TOKEN_PARAMETER": '/hero-of-the-day-dev/slack-token'
}

export class HeroOfTheDayStack extends Stack {
  constructor(scope: Construct, id: string, props?: StackProps) {
    super(scope, id, props);

    let heroTable: ITable = this.heroTable();
    let userTable: ITable = this.userTable();

    let authorizer: IFunction = this.authorizer(heroTable, userTable);
    let heroListFn: IFunction = this.heroList(heroTable);
    let heroGetFn: IFunction = this.heroGet(heroTable);
    let userCreateFn: IFunction = this.userCreate(userTable);

    this.migrate(heroTable, userTable);

    this.apiGateway(authorizer, heroListFn, heroGetFn, userCreateFn);
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

  createFn(id: string, name: string): IFunction {
    return new RustFunction(this, id, {
      package: name,
      environment,
      functionName: `${heroOfTheDay}-${name}`
    });
  }

  migrate(heroTable: ITable, userTable: ITable): IFunction {
    let oldHeroTable = Table.fromTableArn(this, 'OldHeroTable', `arn:aws:dynamodb:eu-central-1:514130831484:table/${environment.OLD_HERO_TABLE}`);
    let oldUserTable = Table.fromTableArn(this, 'OldUserTable', `arn:aws:dynamodb:eu-central-1:514130831484:table/${environment.OLD_USER_TABLE}`);

    let fn = this.createFn('MigrateFunction', 'migrate');
    heroTable.grantReadWriteData(fn);
    userTable.grantReadWriteData(fn);
    oldHeroTable.grantReadData(fn);
    oldUserTable.grantReadData(fn);
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

  userCreate(table: ITable): IFunction {
    let fn = this.createFn('UserCreateFunction', 'user-create');
    table.grantReadWriteData(fn);
    return fn;
  }

  apiGateway(authorizerFn: IFunction, heroListFn: IFunction, heroGetFn: IFunction, userCreateFn: IFunction) {
    const api = new apigw.RestApi(this, `${heroOfTheDay}-api`, {
      description: heroOfTheDay,
      defaultCorsPreflightOptions: {
        statusCode: 200,
        allowHeaders: [
          'Content-Type','X-Amz-Date','Authorization','X-Api-Key','X-Amz-Security-Token','X-Amz-User-Agent'
        ],
        allowOrigins: apigw.Cors.ALL_ORIGINS,
        allowMethods: apigw.Cors.ALL_METHODS,
      }
    });

    new CfnOutput(this, 'apiUrl', { value: api.url });

    let heroPath = api.root.addResource('hero');
    let userPath = api.root.addResource('user');

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

    heroPath.addResource('{hero}').addMethod('GET',
      new apigw.LambdaIntegration(heroGetFn, { proxy: true }), 
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
  }
}
