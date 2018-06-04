FROM node:alpine

RUN apk --update add imagemagick libpng && \
    rm -rf /var/cache/apk/*

COPY package.json package.json
COPY index.js index.js

RUN npm install

ENTRYPOINT ["node", "index.js" ]

CMD [ "--help" ]