#!/bin/bash

cd "$(dirname "$0")"

python3 -m http.server 3000 &
HTTP_PID=$!

trap "kill $HTTP_PID 2>/dev/null" EXIT

echo "HTTP server started at http://127.0.0.1:3000"
echo "Open http://127.0.0.1:3000 in your browser"
echo ""

ffmpeg -f avfoundation -i ":1" -f s16le -ar 16000 -ac 1 - 2>/dev/null | qasr | websocat -t -s 8888
