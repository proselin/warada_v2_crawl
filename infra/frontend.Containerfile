FROM node:22-alpine AS build
WORKDIR /src
COPY package.json package-lock.json ./
COPY ui/ ./ui/
ARG APP_PACKAGE
ARG APP_PATH
ARG BUILD_UI_KIT=false
ARG MF_ACTIVITY_REMOTE_URL
ARG MF_UI_KIT_REMOTE_URL
ENV MF_ACTIVITY_REMOTE_URL=${MF_ACTIVITY_REMOTE_URL}
ENV MF_UI_KIT_REMOTE_URL=${MF_UI_KIT_REMOTE_URL}
RUN --mount=type=cache,target=/root/.npm \
    npm ci --workspace="${APP_PACKAGE}" --workspace=@warada/ui-kit \
      --include-workspace-root=true \
    && if [ "${BUILD_UI_KIT}" = "true" ]; then \
      npm run build --workspace=@warada/ui-kit; \
    fi \
    && npm run build --workspace="${APP_PACKAGE}"

FROM nginx:alpine
ARG APP_PATH
COPY --from=build /src/${APP_PATH}/dist /usr/share/nginx/html
