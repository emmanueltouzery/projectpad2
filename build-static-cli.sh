docker build . --tag ppcli
docker run -v ${HOME}/ppcli_static:/host ppcli:latest
