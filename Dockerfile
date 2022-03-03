FROM rust:1.58.0-alpine AS builder
WORKDIR /usr/src/

RUN apk add --no-cache --update alpine-sdk

COPY ./Cargo.toml ./
COPY ./Cargo.lock ./
COPY ./src/ ./src/
RUN cargo install --path .

FROM alpine:3.15 AS documentation
RUN apk add --no-cache --update scdoc make
COPY ./Makefile ./
COPY ./manual/ ./manual/
RUN make -j$(nproc) doc

FROM alpine:3.15 AS release

RUN apk add --no-cache --update mandoc libgcc
RUN mkdir -p /usr/share/man/man1 /usr/share/man/man3 /usr/share/man/man5
COPY --from=builder /usr/local/cargo/bin/seatrial /bin/seatrial
COPY --from=documentation ./manual/*.1 /usr/share/man/man1/
COPY --from=documentation ./manual/*.3 /usr/share/man/man3/
COPY --from=documentation ./manual/*.5 /usr/share/man/man5/

CMD ["seatrial"]
