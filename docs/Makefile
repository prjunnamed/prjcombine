# Minimal makefile for Sphinx documentation
#

# You can set these variables from the command line, and also
# from the environment for the first two.
SPHINXOPTS    ?=
SPHINXBUILD   ?= sphinx-build
SOURCEDIR     = .
BUILDDIR      = _build

# Put it first so that "make" without argument is like "make help".
help:
	@$(SPHINXBUILD) -M help "$(SOURCEDIR)" "$(BUILDDIR)" $(SPHINXOPTS) $(O)

.PHONY: help Makefile

# Catch-all target: route all unknown targets to Sphinx using the new
# "make mode" option.  $(O) is meant as a shortcut for $(SPHINXOPTS).
html: Makefile generated-xc9500 generated-xpla3 generated-xc2c generated-xilinx
	@$(SPHINXBUILD) -M $@ "$(SOURCEDIR)" "$(BUILDDIR)" $(SPHINXOPTS) $(O)

generated-xc9500: gen_xc9500.py ../databases/xc9500.json ../databases/xc9500xl.json ../databases/xc9500xv.json
	python gen_xc9500.py

generated-xpla3: gen_xpla3.py ../databases/xpla3.json
	python gen_xpla3.py

generated-xc2c: gen_xc2c.py ../databases/xc2c.json
	python gen_xc2c.py

generated-xilinx: gen_xilinx.py ../databases/virtex2.json ../databases/spartan3.json
	python gen_xilinx.py

.PHONY: generated-xc9500 generated-xpla3 generated-xc2c generated-xilinx
