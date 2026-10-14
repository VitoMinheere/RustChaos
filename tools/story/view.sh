#!/bin/sh
# Compact view of a draft: people table on one line, no blank lines.
f="$1"
awk '
/^\| Point/ || /^\|---/ {next}
/^\| [0-9]/ { split($0,a,"|"); gsub(/ /,"",a[2]); t=a[4]; gsub(/^ +| +$/,"",t); r=a[5]; gsub(/^ +| +$/,"",r); n=a[6]; gsub(/^ +| +$/,"",n); ppl=ppl a[2] ":" t "/" r (n!=""?"[" n "]":"") "; "; next }
/^<details>|^<\/details>/ { if (ppl!="") { print "PEOPLE: " ppl; ppl="" } ; next }
NF==0 {next}
{print}
' "$f"
