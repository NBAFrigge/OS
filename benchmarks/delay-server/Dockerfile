FROM golang:alpine AS builder
WORKDIR /app
COPY main.go .
RUN go build -o server main.go

FROM alpine
COPY --from=builder /app/server /server
EXPOSE 80
CMD ["/server"]
