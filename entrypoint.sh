#!/bin/sh
set -e

chown -R prieco:prieco /app/data/tantivy /app/data/blobs /app/data/meta /app/data/vectors

exec runuser -u prieco -- ./prieco_web
