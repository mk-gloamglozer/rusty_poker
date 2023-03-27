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

FROM nginx:stable-alpine AS prod

WORKDIR /app

RUN apk add --update nodejs

COPY frontend/package.json ./package.json
COPY docker/nginx.conf /etc/nginx/conf.d/default.conf

COPY --from=builder /app/build .
COPY target/x86_64-unknown-linux-musl/release/bin .
COPY --from=installer /app/node_modules ./node_modules

COPY <<'EOF' ./start.sh
#!/bin/sh
./bin&
pid_1=$!
node index.js&
pid_2=$!

nginx -g "daemon off;" &
pid_3=$!

trap_handler (){
    if [ -n "${pid_1}" ]; then
        kill -2 ${pid_1}
        wait ${pid_1}
    fi

    if [ -n "${pid_2}" ]; then
        kill -2 ${pid_2}
        wait ${pid_2}
    fi

    if [ -n "${pid_3}" ]; then
        kill -3 ${pid_3}
        wait ${pid_3}
    fi

    exit 0
}
trap trap_handler INT TERM QUIT
wait
EOF

RUN chmod +x ./start.sh

ENV ORIGIN="http://localhost:3000"

ENTRYPOINT ["./start.sh"]