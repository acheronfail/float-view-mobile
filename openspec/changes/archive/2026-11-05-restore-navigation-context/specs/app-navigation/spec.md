## ADDED Requirements

### Requirement: Frontend browsing context

Within one frontend lifetime, routed pages SHALL retain bounded independent browsing positions. Returning through header, native or browser back, or switching primary tabs, SHALL restore retained context after required content loads without flashing the top. New destinations SHALL start at the top. Frontend recreation or cache eviction SHALL use normal initial presentation.

#### Scenario: Deep board history

- **WHEN** a user loads multiple session pages, opens a session and returns by any back control
- **THEN** the board history retains its loaded extent and restores the same visible session at the same viewport offset

#### Scenario: Independent destinations

- **WHEN** a user browses different boards or switches between primary tabs and returns
- **THEN** each retained destination restores its own position without applying another destination's offset

#### Scenario: Keyboard return

- **WHEN** a keyboard user opens a session and returns to its retained list
- **THEN** focus returns to the opened session link without changing the restored position

#### Scenario: New or evicted destination

- **WHEN** a user opens an unvisited destination, or returns after frontend recreation or bounded cache eviction
- **THEN** it renders normally from the top without backend persistence or replaying any action

### Requirement: Browsing restoration with changed content

Retained session lists SHALL reconcile catalog changes through fresh bounded reads, without reusing stale cursors. Restoration SHALL prefer a surviving visible session anchor and otherwise a nearby retained session or clamped position. Failed reads SHALL expose normal retry controls. Restoration SHALL release presentation on user interaction and SHALL NOT override subsequent user scrolling or move a newer destination.

#### Scenario: Changed catalog

- **WHEN** import or deletion changes the catalog while a user views a session
- **THEN** return loads fresh pages through the previously loaded extent and restores a surviving nearby session without resetting to the first page or displaying stale deleted rows

#### Scenario: Failed or interrupted restoration

- **WHEN** loading fails, the user interacts, or another navigation begins during restoration
- **THEN** restoration releases presentation and does not later move the new destination or override user scrolling
