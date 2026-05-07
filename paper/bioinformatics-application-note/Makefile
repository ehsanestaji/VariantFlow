OUTDIR := build
PDF := $(OUTDIR)/variantflow-bioinformatics-note.pdf

.PHONY: all clean

all: $(PDF)

$(PDF): main.tex references.bib sections/*.tex tables/*.tex figures/*.tex
	mkdir -p $(OUTDIR)
	latexmk -pdf -interaction=nonstopmode -halt-on-error -outdir=$(OUTDIR) main.tex
	cp $(OUTDIR)/main.pdf $(PDF)

clean:
	rm -rf $(OUTDIR)
