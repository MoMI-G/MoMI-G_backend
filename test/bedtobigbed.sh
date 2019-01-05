#!/bin/bash

awk '$5>1000{$5=1000;print $0}'
bedToBigBed hgTables_hg19_rmask.sorted.canonical.bed hg19.chrom.sizes repeats.bb


