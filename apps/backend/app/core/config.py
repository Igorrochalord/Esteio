from functools import cached_property

from pydantic_settings import BaseSettings, SettingsConfigDict


class Settings(BaseSettings):
    model_config = SettingsConfigDict(extra="ignore")

    env: str = "dev"
    log_level: str = "INFO"

    database_url: str
    migrations_database_url: str
    # NullPool evita reaproveitar conexões entre event loops (usado nos testes)
    db_null_pool: bool = False
    db_app_role: str = "nucleo_app"
    db_auth_role: str = "nucleo_auth"

    jwt_secret: str
    jwt_algorithm: str = "HS256"
    access_token_ttl_minutes: int = 15
    refresh_token_ttl_days: int = 30

    s3_endpoint_url: str | None = None
    s3_public_endpoint_url: str | None = None
    s3_access_key: str | None = None
    s3_secret_key: str | None = None
    s3_region: str = "us-east-1"
    s3_bucket: str
    presigned_url_ttl_seconds: int = 3600
    logo_max_bytes: int = 2 * 1024 * 1024

    cors_origins: str = ""

    @cached_property
    def cors_origin_list(self) -> list[str]:
        return [o.strip() for o in self.cors_origins.split(",") if o.strip()]


settings = Settings()
