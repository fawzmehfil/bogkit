import { useCallback, useEffect, useRef, useState } from "react";
import type { GameSnapshot, Profile } from "./types";

export function useGameSocket(profile: Profile) {
  const [snapshot, setSnapshot] = useState<GameSnapshot | null>(null);
  const [connected, setConnected] = useState(false);
  const socketRef = useRef<WebSocket | null>(null);
  const profileRef = useRef(profile);
  const retryRef = useRef(0);
  profileRef.current = profile;

  useEffect(() => {
    let stopped = false;
    let timer = 0;
    const connect = () => {
      if (stopped || document.hidden || !navigator.onLine) return;
      const protocol = location.protocol === "https:" ? "wss:" : "ws:";
      const socket = new WebSocket(`${protocol}//${location.host}/ws`);
      socketRef.current = socket;
      socket.onopen = () => {
        if (stopped || socketRef.current !== socket) {
          socket.close();
          return;
        }
        retryRef.current = 0;
        setConnected(true);
        socket.send(JSON.stringify({ type: "register", ...profileRef.current }));
      };
      socket.onmessage = (event) => {
        if (stopped || socketRef.current !== socket) return;
        try { setSnapshot(JSON.parse(event.data) as GameSnapshot); } catch { /* ignore malformed frames */ }
      };
      socket.onclose = () => {
        if (socketRef.current !== socket) return;
        socketRef.current = null;
        setConnected(false);
        if (!stopped && !document.hidden && navigator.onLine) {
          const delays = [800, 1600, 3200, 10_000];
          timer = window.setTimeout(connect, delays[Math.min(retryRef.current++, delays.length - 1)]);
        }
      };
    };
    const resume = () => { if (!stopped && !document.hidden && navigator.onLine && !socketRef.current) connect(); };
    document.addEventListener("visibilitychange", resume);
    window.addEventListener("online", resume);
    connect();
    return () => {
      stopped = true;
      window.clearTimeout(timer);
      document.removeEventListener("visibilitychange", resume);
      window.removeEventListener("online", resume);
      socketRef.current?.close();
      socketRef.current = null;
    };
  }, []);

  useEffect(() => {
    const socket = socketRef.current;
    if (socket?.readyState === WebSocket.OPEN) socket.send(JSON.stringify({ type: "register", ...profile }));
  }, [profile]);

  const send = useCallback((message: Record<string, unknown>) => {
    const socket = socketRef.current;
    if (socket?.readyState === WebSocket.OPEN) socket.send(JSON.stringify(message));
  }, []);

  return { snapshot, connected, send };
}
