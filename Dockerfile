FROM alpine:latest
RUN adduser --disabled-password twitton
RUN apk --no-cache add dumb-init
WORKDIR /home/twitton

COPY twitton /usr/local/bin/twitton
RUN chmod +x /usr/local/bin/twitton

# Run as non-root
USER twitton
ENTRYPOINT ["dumb-init", "/usr/local/bin/twitton"]
