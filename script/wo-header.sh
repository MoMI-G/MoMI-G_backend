#!/bin/bash
# Remove comment-line on GFF3/GTF format.

grep -v "#" $1 > $1.wo.header.gff3
