from app.core import storage
from app.modules.tenants.models import Tenant
from app.modules.tenants.schemas import BrandingOut


def branding_of(tenant: Tenant) -> BrandingOut:
    return BrandingOut(
        name=tenant.name,
        primary_color=tenant.primary_color,
        secondary_color=tenant.secondary_color,
        logo_url=storage.presigned_get_url(tenant.logo_key),
    )
