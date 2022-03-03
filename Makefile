.POSIX:

SEATRIAL_VERSION ?= $(shell git describe --tags)
REGISTRY ?= ghcr.io/dockwa/seatrial
REGISTRY_IMAGE ?= $(SEATRIAL_VERSION)

ifeq ($(PUSH),1)
	PUSH_FLAG := --push
endif

ifeq ($(IS_LATEST),1)
	TAG_LATEST_FLAG := -t $(REGISTRY):latest
endif

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

.PHONY: multiarch-qemu-reset
multiarch-qemu-reset:
	# this is PROBABLY dangerous in the event your system already has bin
	# handlers or something but whatever, those are *super* uncommon normally
	#
	# DO NOT run this on GitHub Actions, use docker/setup-qemu-action@v1
	# instead!
	[ "$$(uname -s)" = "Linux" ] && docker run --rm --privileged multiarch/qemu-user-static --reset -p yes || true

.PHONY: buildx-image
buildx-image:
	docker buildx build \
		--platform linux/amd64,linux/arm64 \
		-t $(REGISTRY):$(REGISTRY_IMAGE) \
		$(TAG_LATEST_FLAG) \
		$(PUSH_FLAG) \
		.
