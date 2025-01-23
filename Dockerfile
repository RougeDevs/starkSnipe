# Use the builder image to compile the Rust application
FROM debian:bookworm-slim AS builder

# Install dependencies including GLIBC 2.33+, OpenSSL, and pkg-config
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    ca-certificates \
    curl \
    wget \
    libc6 \
    protobuf-compiler \
    libprotobuf-dev \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Set the working directory for the build process
WORKDIR /usr/src/app

# Install Rust toolchain
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y
ENV PATH="/root/.cargo/bin:$PATH"

# Copy Cargo files separately to leverage Docker caching
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to build dependencies first
RUN mkdir -p src && echo "fn main() {}" > src/main.rs
RUN cargo build --release

# Copy the actual source code
COPY . .

# Build the final executable
RUN cargo build --release

# Use a minimal base image for the final container
FROM debian:bookworm-slim

# Install required system dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libc6 \
    curl \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Set the working directory to root
WORKDIR /

# Copy the compiled binary from the builder stage
COPY --from=builder /usr/src/app/target/release/meme-sniper /

# Copy the indexer state file if needed (ensure it's writable)
COPY indexer_state.json /
RUN chmod +w /indexer_state.json

# Copy the start.sh script and ensure it's executable
COPY start.sh /start.sh
RUN chmod +x /start.sh
RUN chmod +x /meme-sniper


# Set the entrypoint to the start.sh script
ENTRYPOINT ["start.sh"]

# CMD to run the binary as fallback in case start.sh fails or is not specified
CMD ["meme-sniper"]
