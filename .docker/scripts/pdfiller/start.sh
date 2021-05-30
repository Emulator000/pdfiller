#!/bin/bash

while true; do
    ./pdfiller >> logs/pdfiller.log

    echo "Process crashed with code $?. Restarting..." >&2

    sleep 1
done
