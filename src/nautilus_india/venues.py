"""Indian exchange venues used as the Nautilus ``Venue`` code.

One ClickHouse database serves every venue. The instrument id is
``{symbol}.{venue}``. Cash and derivatives on the same exchange use different
venue codes because their sessions, lot rules, and contract calendars differ.
"""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True, slots=True)
class VenueSpec:
    code: str
    mic: str
    segment: str
    description: str


NSE = VenueSpec("NSE", "XNSE", "CM", "NSE cash market")
BSE = VenueSpec("BSE", "XBOM", "CM", "BSE cash market")
NFO = VenueSpec("NFO", "XNSE", "FO", "NSE equity and index derivatives")
BFO = VenueSpec("BFO", "XBOM", "FO", "BSE equity and index derivatives")
CDS = VenueSpec("CDS", "XNSE", "CD", "NSE currency derivatives")
MCX = VenueSpec("MCX", "XIMC", "COM", "Multi Commodity Exchange")

VENUES: dict[str, VenueSpec] = {
    spec.code: spec for spec in (NSE, BSE, NFO, BFO, CDS, MCX)
}

# Registered together on the live data client so one ClickHouse connection
# can route NSE, BSE, and derivative instruments.
INDIA_VENUE_CODES: tuple[str, ...] = tuple(VENUES)
