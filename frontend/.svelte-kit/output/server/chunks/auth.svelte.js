import "clsx";
const STORAGE_KEY = "fx_auth";
let stored = null;
const raw = localStorage.getItem(STORAGE_KEY);
if (raw) {
  try {
    stored = JSON.parse(raw);
  } catch {
  }
}
let user = stored;
function getAuth() {
  return user;
}
function getToken() {
  return user?.token ?? null;
}
export {
  getToken as a,
  getAuth as g
};
