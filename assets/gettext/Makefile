LANGS = \
	pl \
	en

TARGETS = $(LANGS:%=%/LC_MESSAGES/moniuszko.mo)

%/LC_MESSAGES/moniuszko.mo: %.po
	mkdir -p $(dir $@)
	msgfmt -o $@ $<

all: $(TARGETS)