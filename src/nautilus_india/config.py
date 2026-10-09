"""Connection settings for the ClickHouse market database.

Credentials live on this object. Serializing a Nautilus client config that
embeds these settings also serializes the password.
"""

from __future__ import annotations

from dataclasses import dataclass
import os


@dataclass(frozen=True, slots=True)
class ClickHouseConfig:
    host: str = "localhost"
    port: int = 8123
    username: str = "default"
    password: str = ""
    database: str = "nautilus_india"
    secure: bool = False

    @classmethod
    def from_env(cls) -> ClickHouseConfig:
        secure = os.environ.get("CLICKHOUSE_SECURE", "").lower() in {"1", "true", "yes"}
        return cls(
            host=os.environ.get("CLICKHOUSE_HOST", "localhost"),
            port=int(os.environ.get("CLICKHOUSE_PORT", "8443" if secure else "8123")),
            username=os.environ.get("CLICKHOUSE_USER", "default"),
            password=os.environ.get("CLICKHOUSE_PASSWORD", ""),
            database=os.environ.get("CLICKHOUSE_DATABASE", "nautilus_india"),
            secure=secure,
        )
