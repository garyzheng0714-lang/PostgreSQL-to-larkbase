import axios from "axios";

const API_BASE =
  import.meta.env.VITE_API_BASE_URL ?? "";

const HELPER_API_KEY =
  import.meta.env.VITE_HELPER_API_KEY ?? "";

export const apiClient = axios.create({
  baseURL: API_BASE,
  timeout: 15000,
  headers: {
    "Content-Type": "application/json",
    ...(HELPER_API_KEY ? { "X-Helper-Api-Key": HELPER_API_KEY } : {}),
  },
});
