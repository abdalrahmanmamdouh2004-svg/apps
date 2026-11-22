# Weather

The store's page is [`store.md`](store.md). This is the contract of the widget card, which the watch
(`src/model.rs`, `Model::columns`) and FMC Companion both write: the same forecast at the same hour gives the same
bytes, so change both sides together.

## The card (stub 9, loader of 2026-11-21)

A v2 `columns` record (the SDK's `docs/widget-push.md`) of at most four columns: now, then the three hours after the
one that contains now. Hours that aren't known give no column. Without the current weather the phone writes nothing
and the watch keeps its text card; so does a watch that answers `-22` to the record.

- Caption: `Now`, then `HH:00` (24 h, zero-padded, `00:00` after `23:00`).
- Hours count from the forecast's own first hour and the phone's clock, never from a stale first entry.
- Value: whole degrees C and `°` (U+00B0), no unit. Now is the current record's 0.1° rounded, halves away from zero,
  never `-0`; an hour is its entry's temperature.
- Icon: the index of the condition, from the WMO code of the current record or of that hour. No night variants.

| Index | `widget-icons/` | `model::Icon` | WMO codes |
|---|---|---|---|
| 0 | `00-clear.svg` | `Clear` | 0 |
| 1 | `01-partly.svg` | `Partly` | 1, 2 |
| 2 | `02-cloud.svg` | `Cloud` | anything not listed here |
| 3 | `03-fog.svg` | `Fog` | 45, 48 |
| 4 | `04-drizzle.svg` | `Drizzle` | 51-57 |
| 5 | `05-rain.svg` | `Rain` | 61-67, 80-82 |
| 6 | `06-snow.svg` | `Snow` | 71-77, 85, 86 |
| 7 | `07-storm.svg` | `Storm` | 95-99 |

The index is the file's place in name order and is stored on watches and sent by phones: append, never reorder.

Test vector (87 bytes, `src/model_tests.rs`, `columns_golden_bytes`):
`wp2apps.install.columns_record([("Now",6,"-3°"),("23:00",2,"-4°"),("00:00",0,"-5°"),("01:00",7,"0°")])`.
