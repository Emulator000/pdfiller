#!/bin/sh

while true; do
    ./pdfiller | awk '{ print strftime("%Y-%m-%d %H:%M:%S"), $0; fflush(); }' >> logs/pdfiller.log

    echo "Process crashed with code $?. Restarting..." >&2

    sleep 1
done
