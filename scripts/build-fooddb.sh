#!/usr/bin/env bash
# Download USDA FoodData Central (SR Legacy + Foundation, public domain) and UK CoFID
# (Open Government Licence) and build
# apps/desktop/resources/foods.sqlite, the food database shipped inside the app.
set -euo pipefail
cd "$(dirname "$0")/.."
SR=FoodData_Central_sr_legacy_food_csv_2018-04
FND=FoodData_Central_foundation_food_csv_2026-10-29
WORK="${FOODDB_WORK:-target/fooddb}"
mkdir -p "$WORK"
for f in $SR $FND; do
  if [ ! -f "$WORK/$f/food.csv" ]; then
    curl -fsSL --retry 3 -o "$WORK/$f.zip" "https://fdc.nal.usda.gov/fdc-datasets/$f.zip"
    unzip -qo "$WORK/$f.zip" -d "$WORK"
  fi
done
# UK CoFID 2021, Open Government Licence v3.0
COFID_URL="https://assets.publishing.service.gov.uk/media/60538b91e90e07527df82ae4/McCance_Widdowsons_Composition_of_Foods_Integrated_Dataset_2021..xlsx"
[ -s "$WORK/cofid-2021.xlsx" ] || curl -fsSL --retry 3 -o "$WORK/cofid-2021.xlsx" "$COFID_URL"
cargo run --release -q -p privatei-core --features import --bin build-fooddb -- \
  --out apps/desktop/resources/foods.sqlite --version "SR Legacy 2018-04 + Foundation 2026-10" \
  --cofid "$WORK/cofid-2021.xlsx" "$WORK/$SR" "$WORK/$FND"
