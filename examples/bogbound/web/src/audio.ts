let context: AudioContext | null = null;
function tone(frequency:number,duration:number,type:OscillatorType="square",volume=0.035){
  context ??= new AudioContext();
  if (context.state === "suspended") void context.resume();
  const oscillator=context.createOscillator();const gain=context.createGain();oscillator.type=type;oscillator.frequency.value=frequency;
  gain.gain.setValueAtTime(volume,context.currentTime);gain.gain.exponentialRampToValueAtTime(0.0001,context.currentTime+duration);
  oscillator.connect(gain).connect(context.destination);oscillator.start();oscillator.stop(context.currentTime+duration);
}
export function playCue(cue:"wave"|"level"|"rune"|"victory"|"defeat"){
  if(cue==="wave"){tone(196,.12);setTimeout(()=>tone(294,.18),100)}
  else if(cue==="level"){tone(392,.1);setTimeout(()=>tone(523,.16),90)}
  else if(cue==="rune"){tone(330,.12,"triangle");setTimeout(()=>tone(660,.2,"triangle"),100)}
  else if(cue==="victory"){[262,330,392,523].forEach((f,i)=>setTimeout(()=>tone(f,.28),i*120))}
  else tone(110,.45,"sawtooth",.025);
}
