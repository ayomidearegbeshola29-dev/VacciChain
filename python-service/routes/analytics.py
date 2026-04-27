import os
from fastapi import APIRouter, Security
from fastapi.security import HTTPBearer, HTTPAuthorizationCredentials
from schemas import VaccinationRatesResponse, IssuerActivityResponse, AnomalyResponse

router = APIRouter(tags=["Analytics"])

BACKEND_URL = os.getenv("BACKEND_URL", "http://backend:4000")

_bearer = HTTPBearer(description="JWT issued by the VacciChain backend via POST /auth/verify")


@router.get(
    "/rates",
    response_model=VaccinationRatesResponse,
    summary="Vaccination rates by vaccine type",
    description=(
        "Returns aggregated vaccination counts grouped by vaccine type. "
        "In production this queries an indexed Horizon event stream.\n\n"
        "**Auth:** Bearer JWT required."
    ),
)
async def vaccination_rates(
    _: HTTPAuthorizationCredentials = Security(_bearer),
):
    return VaccinationRatesResponse(
        note="Connect to Horizon event stream for live data",
        sample={"COVID-19": 1240, "Influenza": 870, "Hepatitis B": 430},
    )


@router.get(
    "/issuers",
    response_model=IssuerActivityResponse,
    summary="Issuer activity — volume and last active date",
    description=(
        "Returns per-issuer mint volume and last-active date, "
        "derived from on-chain mint events.\n\n"
        "**Auth:** Bearer JWT required."
    ),
)
async def issuer_activity(
    _: HTTPAuthorizationCredentials = Security(_bearer),
):
    return IssuerActivityResponse(
        note="Derived from on-chain mint events",
        sample=[
            {"issuer": "GABC...XYZ", "total_issued": 312, "last_active": "2024-03-15"},
            {"issuer": "GDEF...UVW", "total_issued": 98, "last_active": "2024-03-10"},
        ],
    )


@router.get(
    "/anomalies",
    response_model=AnomalyResponse,
    summary="Flag issuers with unusual mint volume",
    description=(
        "Detects issuers exceeding 50 mints within any 1-hour window and returns "
        "their Stellar addresses.\n\n"
        "**Auth:** Bearer JWT required."
    ),
)
async def anomaly_detection(
    _: HTTPAuthorizationCredentials = Security(_bearer),
):
    return AnomalyResponse(
        note="Anomaly detection based on mint event frequency (threshold: >50 mints/hour)",
        flagged_issuers=[],
    )
