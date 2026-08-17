import { useEffect, useRef } from "react";
import Phaser from "phaser";
import { BogboundScene } from "./GameScene";
import type { GameSnapshot, InputVector } from "./types";

export function GameCanvas({snapshot,input}:{snapshot:GameSnapshot;input:InputVector}){
  const hostRef=useRef<HTMLDivElement>(null);const gameRef=useRef<Phaser.Game|null>(null);const sceneRef=useRef<BogboundScene|null>(null);const runningRef=useRef(snapshot.phase==="running");runningRef.current=snapshot.phase==="running";
  useEffect(()=>{if(!hostRef.current||gameRef.current)return;const scene=new BogboundScene();sceneRef.current=scene;gameRef.current=new Phaser.Game({type:Phaser.AUTO,parent:hostRef.current,width:480,height:270,backgroundColor:"#2d7750",pixelArt:true,antialias:false,roundPixels:true,fps:{target:30,forceSetTimeOut:true},render:{antialias:false,antialiasGL:false,pixelArt:true,roundPixels:true,powerPreference:"low-power"},scale:{mode:Phaser.Scale.RESIZE,width:"100%",height:"100%"},scene:[scene]});const visibility=()=>{if(document.hidden||!runningRef.current)gameRef.current?.loop.sleep();else gameRef.current?.loop.wake()};document.addEventListener("visibilitychange",visibility);return()=>{document.removeEventListener("visibilitychange",visibility);gameRef.current?.destroy(true);gameRef.current=null;sceneRef.current=null}},[]);
  useEffect(()=>sceneRef.current?.setSnapshot(snapshot),[snapshot]);
  useEffect(()=>{if(document.hidden||snapshot.phase!=="running")gameRef.current?.loop.sleep();else gameRef.current?.loop.wake()},[snapshot.phase]);
  useEffect(()=>sceneRef.current?.setInput(input),[input]);
  return <div className="game-canvas" ref={hostRef} aria-label="Bogbound game world"/>;
}
