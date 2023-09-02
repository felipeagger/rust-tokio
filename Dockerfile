FROM rust:1.71.1 as build

RUN apt-get update -yqq && apt-get install -yqq cmake g++

WORKDIR /app

#COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

RUN cargo build --release

COPY ./src ./src

# build for release
RUN cargo build --release --bin bin

#Second stage
FROM rust:1.59-slim-buster

COPY --from=build /app/bin .

CMD ["./app"]