import axios from "axios";

const API_BASE =
  import.meta.env.VITE_API_BASE_URL ?? "";

export const apiClient = axios.create({
  baseURL: API_BASE,
  timeout: 15000,
  headers: { "Content-Type": "application/json" },
});
