import { renderOverlay, type OverlayState } from "./render";
import "./styles.css";

const root = document.getElementById("overlay");
if (!root) throw new Error("Overlay root is missing");

const token = new URLSearchParams(location.search).get("token") ?? "";
const tokenQuery = token ? `?token=${encodeURIComponent(token)}` : "";

async function loadProgramSnapshot() {
  const response = await fetch(`/api/overlay/program/snapshot${tokenQuery}`);
  if (!response.ok) return;
  const snapshot = await response.json();
  renderProgramSnapshot(snapshot);
}

function connectProgramSocket() {
  const scheme = location.protocol === "https:" ? "wss" : "ws";
  const socket = new WebSocket(
    `${scheme}://${location.host}/ws/overlay/program${tokenQuery}`,
  );
  socket.onmessage = (event) => {
    const snapshot = JSON.parse(event.data) as {
      overlayState?: OverlayState;
    } | null;
    renderProgramSnapshot(snapshot, socket);
  };
  socket.onclose = () => {
    window.setTimeout(connectProgramSocket, 1000);
  };
}

function renderProgramSnapshot(
  snapshot: { overlayState?: OverlayState } | null,
  socket?: WebSocket,
) {
  const state = snapshot?.overlayState ?? null;
  renderOverlay(root!, state);
  if (socket?.readyState === WebSocket.OPEN) {
    socket.send(
      JSON.stringify({
        event: "overlay.rendered",
        programRevision: state?.programRevision,
      }),
    );
  }
}

void loadProgramSnapshot();
connectProgramSocket();
