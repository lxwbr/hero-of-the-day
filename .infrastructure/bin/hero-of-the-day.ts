#!/usr/bin/env node
import 'source-map-support/register';
import * as cdk from 'aws-cdk-lib';
import { HeroOfTheDayStack } from '../lib/hero-of-the-day-stack';
import {Annotations} from "aws-cdk-lib";

const app = new cdk.App();

let APP_NAME = 'hero-of-the-day';
let HOSTED_DOMAIN = process.env.HOSTED_DOMAIN;
let MS_CLIENT_ID = process.env.MS_CLIENT_ID;
if (!HOSTED_DOMAIN) {
  Annotations.of(app).addError('Could not determine HOSTED_DOMAIN');
  throw Error('Could not determine HOSTED_DOMAIN')
}
if (!MS_CLIENT_ID) {
  Annotations.of(app).addError('Could not determine MS_CLIENT_ID');
  throw Error('Could not determine MS_CLIENT_ID')
}

new HeroOfTheDayStack(app, 'HeroOfTheDayStack', {
  APP_NAME,
  HOSTED_DOMAIN,
  MS_CLIENT_ID,
  HERO_TABLE: `${APP_NAME}-hero`,
  USER_TABLE: `${APP_NAME}-user`,
  SCHEDULE_TABLE: `${APP_NAME}-schedule`,
  PUNCH_CLOCK_TABLE: `${APP_NAME}-punch-clock`,
  SLACK_TOKEN_PARAMETER: `/${APP_NAME}/slack-token`
});
