from fastapi import FastAPI
from routes.analytics import router as analytics_router
from routes.batch import router as batch_router
from schemas import HealthResponse

app = FastAPI(
    title="VacciChain Analytics",
    version="1.0.0",
    description=(
        "Analytics and batch-verification service for the VacciChain platform.\n\n"
        "**Authentication:** Analytics endpoints (`/analytics/*`) require a valid JWT "
        "issued by the VacciChain backend (`Authorization: Bearer <token>`). "
        "The `/batch/verify` endpoint is public. "
        "The `/health` endpoint is public."
    ),
    docs_url="/docs",
    redoc_url="/redoc",
)

app.include_router(analytics_router, prefix="/analytics")
app.include_router(batch_router, prefix="/batch")


@app.get("/health", response_model=HealthResponse, tags=["Health"])
def health():
    return HealthResponse(status="ok")
