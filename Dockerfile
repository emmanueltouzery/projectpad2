# -*- mode: dockerfile -*-
#
# An example Dockerfile showing how to add new static C libraries using
# musl-gcc.

FROM ekidd/rust-musl-builder:1.49.0

# https://rtfm.co.ua/en/docker-configure-tzdata-and-timezone-during-build/
ENV TZ=Europe/Ljubljana
RUN sudo sh -c "ln -snf /usr/share/zoneinfo/$TZ /etc/localtime && echo $TZ > /etc/timezone"

RUN sudo apt update

RUN sudo apt install tcl -y

# Build a static copy of sqlcipher.
# https://github.com/sqlcipher/sqlcipher/issues/132#issuecomment-122908672 
# also related https://discuss.zetetic.net/t/cross-compile-sqlicipher-for-arm/2104/4
# https://github.com/sqlcipher/sqlcipher/issues/276
# https://github.com/rust-lang/rust/issues/40049
RUN VERS=4.4.1 && \
    cd /home/rust/libs && \
    curl -LO https://github.com/sqlcipher/sqlcipher/archive/v$VERS.tar.gz && \
    tar xzf v$VERS.tar.gz && cd sqlcipher-$VERS && \
    CC=musl-gcc ./configure  --host=x86_64-pc-linux-gnu --target=x86_64-linux-musl --prefix=/usr/local/musl --disable-tcl --disable-shared --with-crypto-lib=none --enable-static=yes --enable-tempstore=yes CFLAGS="-DSQLITE_HAS_CODEC -DSQLCIPHER_CRYPTO_OPENSSL -I/usr/include/x86_64-linux-musl -I/usr/local/musl/include -I/usr/local/musl/include/openssl" LDFLAGS=" /usr/local/musl/lib/libcrypto.a" && \
    make && sudo make install && \
    cd .. && rm -rf v$VERS.tar.gz sqlcipher-$VERS

ADD --chown=rust:rust ./ .

# https://stackoverflow.com/questions/40695010/how-to-compile-a-static-musl-binary-of-a-rust-project-with-native-dependencies
# https://github.com/rust-lang/rust/issues/54243

ENV RUSTFLAGS='-L/usr/local/musl/lib  -L/usr/lib/x86_64-linux-musl  -L/lib/x86_64-linux-musl -C linker=musl-gcc -Clink-arg=/usr/local/musl/lib/libcrypto.a -Clink-arg=/usr/local/musl/lib/libsqlcipher.a -Clink-arg=/usr/lib/x86_64-linux-musl/libc.a'
CMD cargo build --target x86_64-unknown-linux-musl --release --bin ppcli && cp /home/rust/src/target/x86_64-unknown-linux-musl/release/ppcli /host
