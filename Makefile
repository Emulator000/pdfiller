SHELL=bash

include .env

VARS:=$(shell sed -ne 's/ *\#.*$$//; /./ s/=.*$$// p' .env )
$(foreach v,$(VARS),$(eval $(shell echo export $(v)="$($(v))")))

start_local:
	./target/debug/pdfiller

start:
	./pdfiller.sh start

start_recreate:
	./pdfiller.sh start dev force-recreate

start_prod:
	./pdfiller.sh start prod

start_prod_recreate:
	./pdfiller.sh start dev force-recreate

stop:
	./pdfiller.sh stop

down:
	./pdfiller.sh down
