# Build stage
FROM node:24-alpine AS builder
WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY . .
ARG VITE_API_BASE=""
ENV VITE_API_BASE=$VITE_API_BASE
ARG VITE_USE_API_PROXY="true"
ENV VITE_USE_API_PROXY=$VITE_USE_API_PROXY
RUN npm run build

# Production stage — serve static files and proxy API calls
FROM nginx:alpine
COPY nginx.conf /etc/nginx/conf.d/default.conf
COPY --from=builder /app/dist /usr/share/nginx/html
EXPOSE 3170
