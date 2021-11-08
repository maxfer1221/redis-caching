FROM rust:1-alpine3.14

WORKDIR /code
EXPOSE 8000
RUN apk add --no-cache gcc musl-dev linux-headers
COPY . .

CMD ["cargo", "run"]
