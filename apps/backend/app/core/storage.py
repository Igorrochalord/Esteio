"""Acesso a armazenamento S3 compatível (SeaweedFS em dev, AWS S3 em produção).

O banco guarda apenas a CHAVE do objeto. URLs para o cliente são pré-assinadas
com S3_PUBLIC_ENDPOINT_URL (o endpoint interno do compose não é acessível pelo
navegador).
"""

import asyncio
from functools import lru_cache
from typing import TYPE_CHECKING

import boto3
from botocore.config import Config

from app.core.config import settings

if TYPE_CHECKING:
    from mypy_boto3_s3 import S3Client


def _client(endpoint: str | None) -> "S3Client":
    return boto3.client(
        "s3",
        endpoint_url=endpoint,
        aws_access_key_id=settings.s3_access_key,
        aws_secret_access_key=settings.s3_secret_key,
        region_name=settings.s3_region,
        config=Config(signature_version="s3v4", s3={"addressing_style": "path"}),
    )


@lru_cache
def internal_client() -> "S3Client":
    return _client(settings.s3_endpoint_url)


@lru_cache
def public_client() -> "S3Client":
    return _client(settings.s3_public_endpoint_url or settings.s3_endpoint_url)


async def put_object(key: str, data: bytes, content_type: str) -> None:
    await asyncio.to_thread(
        internal_client().put_object,
        Bucket=settings.s3_bucket,
        Key=key,
        Body=data,
        ContentType=content_type,
    )


async def delete_object(key: str) -> None:
    await asyncio.to_thread(internal_client().delete_object, Bucket=settings.s3_bucket, Key=key)


def presigned_get_url(key: str | None) -> str | None:
    if not key:
        return None
    return public_client().generate_presigned_url(
        "get_object",
        Params={"Bucket": settings.s3_bucket, "Key": key},
        ExpiresIn=settings.presigned_url_ttl_seconds,
    )


async def ping() -> bool:
    try:
        await asyncio.to_thread(internal_client().head_bucket, Bucket=settings.s3_bucket)
        return True
    except Exception:
        return False
