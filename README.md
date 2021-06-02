# hero-of-the-day

This service integrates with Slack and makes scheduled assignments and rotation of users to user-groups possible.

## Build

In the root directory:

```
cargo build
```

## Deployment

```
GOOGLE_CLIENT_ID="XXXXXXXXXXXX.apps.googleusercontent.com" HOSTED_DOMAIN="your-domain.io" serverless deploy
```
