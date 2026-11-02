"""Erros de domínio. Services lançam estes; o main.py traduz para HTTP."""


class DomainError(Exception):
    status_code = 400
    code = "domain_error"

    def __init__(self, message: str) -> None:
        super().__init__(message)
        self.message = message


class NotFoundError(DomainError):
    status_code = 404
    code = "not_found"


class ConflictError(DomainError):
    status_code = 409
    code = "conflict"


class PermissionDeniedError(DomainError):
    status_code = 403
    code = "forbidden"


class AuthenticationError(DomainError):
    status_code = 401
    code = "unauthorized"


class BusinessRuleError(DomainError):
    status_code = 422
    code = "business_rule"
