#!/usr/bin/env bash

# Configure the AWS CLI for the local Garage S3 API.
: "${GARAGE_DEFAULT_ACCESS_KEY:?Missing Garage access key}"
: "${GARAGE_DEFAULT_SECRET_KEY:?Missing Garage secret key}"

umask 077
aws configure set aws_access_key_id "$GARAGE_DEFAULT_ACCESS_KEY"
aws configure set aws_secret_access_key "$GARAGE_DEFAULT_SECRET_KEY"
aws configure set region garage
aws configure set output json
aws configure set endpoint_url http://127.0.0.1:3900
aws configure set s3.addressing_style path
