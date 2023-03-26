FROM node:16.17-alpine AS builder

WORKDIR /app
COPY frontend .
RUN yarn
RUN yarn build

FROM node:16.17 AS prod

WORKDIR /app

COPY frontend/package.json ./package.json
RUN yarn --production

COPY --from=builder /app/build .
COPY target/x86_64-unknown-linux-gnu/release/bin .

RUN echo "./bin& node index.js" > ./start.sh

RUN chmod +x ./start.sh

ENV ORIGIN="http://localhost:3000"

CMD ["./start.sh"]