.POSIX:

.SUFFIXES: .1.scd .1 .3.scd .3 .5.scd .5

# teach make how to build man pages (thanks section 1.11 of
# http://khmere.com/freebsd_book/html/ch01.html)
.1.scd.1:
	scdoc < $< > $@ || (rm -f $@; exit 1)
.3.scd.3:
	scdoc < $< > $@ || (rm -f $@; exit 1)
.5.scd.5:
	scdoc < $< > $@ || (rm -f $@; exit 1)

.PHONY: clean
clean:
	rm -rf manual/*.1 manual/*.3 manual/*.5

doc: manual
manual: manual/seatrial.1 manual/seatrial.5 manual/seatrial.lua.3
