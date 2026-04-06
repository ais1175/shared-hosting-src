import { type NextRequest, NextResponse } from "next/server";

const RUST_API_BASE = process.env.RUST_API_BASE_URL ?? "http://127.0.0.1:8081";

async function forward(request: NextRequest, method: string, pathParts: string[]) {
  const upstream = new URL(`${RUST_API_BASE}/${pathParts.join("/")}`);
  request.nextUrl.searchParams.forEach((value, key) => {
    upstream.searchParams.set(key, value);
  });

  const headers = new Headers();
  request.headers.forEach((value, key) => {
    if (key.toLowerCase() === "host") return;
    headers.set(key, value);
  });

  const hasBody = method !== "GET" && method !== "HEAD";
  const body = hasBody ? await request.text() : undefined;

  try {
    const response = await fetch(upstream, {
      method,
      headers,
      body,
      cache: "no-store",
    });

    const responseBody = await response.text();
    const nextResponse = new NextResponse(responseBody, {
      status: response.status,
      statusText: response.statusText,
    });

    response.headers.forEach((value, key) => {
      if (key.toLowerCase() === "content-encoding") return;
      nextResponse.headers.set(key, value);
    });

    return nextResponse;
  } catch (error) {
    return NextResponse.json(
      {
        ok: false,
        error_code: "API_ERROR",
        message: "API error",
      },
      { status: 502 },
    );
  }
}

export async function GET(
  request: NextRequest,
  { params }: { params: Promise<{ path: string[] }> },
) {
  const { path } = await params;
  return forward(request, "GET", path);
}

export async function POST(
  request: NextRequest,
  { params }: { params: Promise<{ path: string[] }> },
) {
  const { path } = await params;
  return forward(request, "POST", path);
}

export async function OPTIONS() {
  return new NextResponse(null, { status: 204 });
}
