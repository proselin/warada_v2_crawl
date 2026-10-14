FROM node:22-alpine AS build
WORKDIR /src
COPY package.json package-lock.json ./
COPY ui/ ./ui/
RUN --mount=type=cache,target=/root/.npm \
    npm ci --workspace=@warada/ui-kit --include-workspace-root=true \
    && npm run build --workspace=@warada/ui-kit

FROM nginx:alpine
COPY --from=build /src/ui/libs/ui-kit/dist/mf/ /usr/share/nginx/html/
