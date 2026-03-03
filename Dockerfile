# Build frontend
FROM node:20-alpine AS build-frontend
WORKDIR /app/frontend
# Explicitly copy package files first for better caching
COPY frontend/package.json ./
RUN npm install
COPY frontend .
RUN npm run build

# Build backend
FROM rust:1.85-alpine AS build-backend
RUN apk add --no-cache musl-dev
WORKDIR /app/backend
COPY backend/Cargo.toml backend/Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src
COPY backend .
RUN touch src/main.rs && cargo build --release

# Final runtime image
FROM alpine:3.19
RUN apk add --no-cache libgcc
WORKDIR /app
COPY --from=build-backend /app/backend/target/release/backend_v2 .
# The backend binary expects the dist folder at ../frontend/dist relative to its execution path
COPY --from=build-frontend /app/frontend/dist ./frontend/dist
COPY backend/migrations ./migrations

ENV PORT=8082
ENV HOST=0.0.0.0

EXPOSE 8082
CMD ["./backend_v2"]
