docker build . --tag ppcli
docker run -v ${HOME}/ppcli_static:/host ppcli:latest
echo "A ppcli static binary was generated in ${HOME}/ppcli_static"
