# Build frontend
FROM node:20-slim AS build-frontend
# Set a higher memory limit for Node.js to prevent silent OOM kills during Vite build
ENV NODE_OPTIONS="--max-old-space-size=4096"
WORKDIR /app/frontend
# Explicitly copy package files first for better caching
COPY frontend/package.json ./
RUN npm install
COPY frontend .
RUN npm run build

# Build backend
FROM rust:slim AS build-backend
WORKDIR /app/backend
COPY backend/Cargo.toml backend/Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src
COPY backend .
RUN touch src/main.rs && cargo build --release

# Final runtime image
FROM debian:bookworm-slim
# Install libssl and ca-certificates which might be needed by SQLx/Actix
RUN apt-get update && apt-get install -y ca-certificates libssl-dev && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=build-backend /app/backend/target/release/backend_v2 .
# The backend binary expects the dist folder at ../frontend/dist relative to its execution path
COPY --from=build-frontend /app/frontend/dist ./frontend/dist
COPY backend/migrations ./migrations

ENV PORT=8082
ENV HOST=0.0.0.0

EXPOSE 8082
CMD ["./backend_v2"]
