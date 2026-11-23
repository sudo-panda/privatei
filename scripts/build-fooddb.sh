#!/usr/bin/env bash
# Download USDA FoodData Central (SR Legacy + Foundation, public domain) and UK CoFID
# (Open Government Licence) and build
# apps/desktop/resources/foods.sqlite, the food database shipped inside the app.
set -euo pipefail
cd "$(dirname "$0")/.."
SR=FoodData_Central_sr_legacy_food_csv_2018-04
# Newest Foundation release first. USDA has replaced a release in place with an incomplete
# zip before (2026-10-29 lost food.csv on 2026-11-23), so each one is checked and the
# next older one is used if it's missing the tables we read.
FND_RELEASES="2026-10-29 2026-04-30"
WORK="${FOODDB_WORK:-target/fooddb}"
mkdir -p "$WORK"

# fetch NAME: download and unpack NAME.zip unless already there; fail if it lacks a table.
fetch() {
  local f=$1
  if [ ! -f "$WORK/$f/food.csv" ]; then
    curl -fsSL --retry 3 -o "$WORK/$f.zip" "https://fdc.nal.usda.gov/fdc-datasets/$f.zip" || return 1
    unzip -qo "$WORK/$f.zip" -d "$WORK" || return 1
  fi
  for t in food food_nutrient nutrient food_portion; do
    [ -s "$WORK/$f/$t.csv" ] || { echo "USDA $f has no $t.csv" >&2; return 1; }
  done
}

fetch "$SR"
FND=""
for r in $FND_RELEASES; do
  if fetch "FoodData_Central_foundation_food_csv_$r"; then
    FND="FoodData_Central_foundation_food_csv_$r"
    FND_DATE=${r%-*}
    break
  fi
  echo "Skipping Foundation $r; trying an older release." >&2
done
[ -n "$FND" ] || { echo "No complete USDA Foundation release found." >&2; exit 1; }
# UK CoFID 2021, Open Government Licence v3.0
COFID_URL="https://assets.publishing.service.gov.uk/media/60538b91e90e07527df82ae4/McCance_Widdowsons_Composition_of_Foods_Integrated_Dataset_2021..xlsx"
[ -s "$WORK/cofid-2021.xlsx" ] || curl -fsSL --retry 3 -o "$WORK/cofid-2021.xlsx" "$COFID_URL"
cargo run --release -q -p privatei-core --features import --bin build-fooddb -- \
  --out apps/desktop/resources/foods.sqlite --version "SR Legacy 2018-04 + Foundation $FND_DATE" \
  --cofid "$WORK/cofid-2021.xlsx" "$WORK/$SR" "$WORK/$FND"
