# docker build \
#   --build-arg PUBLIC_API_URL=https://api.example.com \
#   --build-arg PUBLIC_GATEWAY_URL=wss://gateway.example.com/gateway \
#   -t pulsar-web .

FROM node:22-slim AS builder
WORKDIR /app
RUN npm install -g pnpm
COPY package.json pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile
COPY . .
ARG PUBLIC_API_URL=http://localhost:3000
ARG PUBLIC_GATEWAY_URL=ws://localhost:3001/gateway
ENV PUBLIC_API_URL=$PUBLIC_API_URL
ENV PUBLIC_GATEWAY_URL=$PUBLIC_GATEWAY_URL
RUN pnpm build

FROM nginx:alpine
COPY --from=builder /app/build /usr/share/nginx/html
COPY nginx.conf /etc/nginx/conf.d/default.conf
EXPOSE 80
