import { useRef } from "react";
import type { InputVector } from "./types";

export function Joystick({onChange}:{onChange:(value:InputVector)=>void}){
  const baseRef=useRef<HTMLDivElement>(null);const pointerRef=useRef<number|null>(null);
  const move=(clientX:number,clientY:number)=>{const rect=baseRef.current?.getBoundingClientRect();if(!rect)return;const dx=clientX-(rect.left+rect.width/2),dy=clientY-(rect.top+rect.height/2);const radius=rect.width*.38;const length=Math.hypot(dx,dy);const scale=length>radius?radius/length:1;onChange({x:dx*scale/radius,y:dy*scale/radius});const knob=baseRef.current?.firstElementChild as HTMLElement|null;if(knob)knob.style.transform=`translate(${dx*scale}px, ${dy*scale}px)`};
  const reset=()=>{pointerRef.current=null;onChange({x:0,y:0});const knob=baseRef.current?.firstElementChild as HTMLElement|null;if(knob)knob.style.transform="translate(0, 0)"};
  return <div className="joystick" ref={baseRef} onPointerDown={e=>{pointerRef.current=e.pointerId;e.currentTarget.setPointerCapture(e.pointerId);move(e.clientX,e.clientY)}} onPointerMove={e=>{if(pointerRef.current===e.pointerId)move(e.clientX,e.clientY)}} onPointerUp={reset} onPointerCancel={reset} aria-label="Movement joystick"><span/></div>;
}
