# syntax=docker/dockerfile:1
FROM node:16.17-alpine AS builder

WORKDIR /app
COPY frontend .
RUN yarn
RUN yarn build

FROM node:16.17-alpine as installer

WORKDIR /app

COPY frontend/package.json ./package.json
RUN yarn --production

FROM node:16.17-alpine AS prod

WORKDIR /app

COPY frontend/package.json ./package.json

COPY --from=builder /app/build .
COPY target/x86_64-unknown-linux-musl/release/bin .
COPY --from=installer /app/node_modules ./node_modules

COPY <<'EOF' ./start.sh
#!/bin/sh
./bin&
pid_1=$!
node index.js&
pid_2=$!

trap_handler (){
    kill -2 ${pid_1} ${pid_2}
    wait ${pid_1}
    wait ${pid_2}
    exit 0
}
trap trap_handler INT TERM
wait
EOF

RUN chmod +x ./start.sh

ENV ORIGIN="http://localhost:3000"

ENTRYPOINT ["./start.sh"]