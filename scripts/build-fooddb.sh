#!/usr/bin/env bash
# Download USDA FoodData Central (SR Legacy + Foundation, public domain) and build
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
cargo run --release -q -p privatei-core --features import --bin build-fooddb -- \
  --out apps/desktop/resources/foods.sqlite --version "SR Legacy 2018-04 + Foundation 2026-10" \
  "$WORK/$SR" "$WORK/$FND"
