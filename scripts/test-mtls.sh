#! /bin/bash

set -ex # exit on error, print commands

if [ -d logs ]; then
    if [ -d logs.bkp ]; then
        rm -rf logs.bkp
    fi
    mv logs logs.bkp
fi
mkdir -p logs

PORT=8443
if lsof -i :$PORT; then
    echo "Port $PORT is already in use. Please kill the process using this port."
    exit 1
fi

cargo run -- server > logs/server.txt 2>&1 &
SERVER_PID=$!
echo "Server PID: $SERVER_PID"

# wait for the server to start listening
while ! grep -q "listening" logs/server.txt; do
    sleep 0.1
done

cargo run -- client 2>&1 | tee logs/client.txt 

kill "$SERVER_PID"