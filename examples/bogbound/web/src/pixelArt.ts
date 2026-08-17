import Phaser from "phaser";
import type { Element, EnemyKind, Skin } from "./types";

type Ctx = CanvasRenderingContext2D;
const P={ink:"#13211c",deep:"#091511",cream:"#f5e5b2",grass:"#4d964d",grassDark:"#28633c",grassLight:"#7abe55",sand:"#d6a85e",sandLight:"#f1cf82",water:"#2b8e91",waterLight:"#63c0ae",wood:"#8c5737",stone:"#68756b",red:"#ef6b4f",orange:"#f69a45",yellow:"#f3d05e",teal:"#43b4a4",blue:"#518ec7",violet:"#8c6ac2",green:"#54a954",pink:"#d85e86",white:"#fff4d0"};
const skinColors:Record<Skin,[string,string,string]>={sun:[P.orange,P.yellow,"#a14f32"],tide:[P.teal,P.blue,"#d9c681"],storm:[P.yellow,P.violet,"#50528e"],grove:[P.green,P.pink,"#386b3d"]};
const elementColors:Record<Element,string>={ember:P.orange,frost:"#9be1da",storm:P.yellow,thorn:P.green};

function make(scene:Phaser.Scene,key:string,w:number,h:number,draw:(c:Ctx)=>void){if(scene.textures.exists(key))return;const tex=scene.textures.createCanvas(key,w,h);if(!tex)throw new Error(`texture ${key}`);tex.context.imageSmoothingEnabled=false;tex.context.clearRect(0,0,w,h);draw(tex.context);tex.refresh()}
function box(c:Ctx,color:string,x:number,y:number,w:number,h:number){c.fillStyle=color;c.fillRect(x,y,w,h)}

function drawGround(c:Ctx,variant:number){box(c,P.grass,0,0,16,16);box(c,P.grassDark,variant+2,11,1,3);box(c,P.grassLight,11-variant,4,2,1);box(c,"#3d8444",6,14,1,1);if(variant===2){box(c,P.sand,1,1,4,3);box(c,P.sandLight,2,1,2,1)}}
function drawPlayer(c:Ctx,skin:Skin,direction:string,frame:number){const [main,accent,dark]=skinColors[skin];const bob=frame?1:0;box(c,P.deep,7,18+bob,4,3);box(c,P.deep,14,18-bob,4,3);box(c,P.ink,5,7+bob,14,13);box(c,"#b87550",7,8+bob,10,8);box(c,"#e4a477",8,8+bob,8,6);box(c,main,6,5+bob,12,6);box(c,dark,7,4+bob,10,3);box(c,accent,10,3+bob,5,3);box(c,main,6,14+bob,12,6);box(c,accent,8,15+bob,8,2);if(skin==="sun"){box(c,P.sandLight,3,5+bob,18,3);box(c,P.wood,18,10+bob,2,8)}else if(skin==="tide"){box(c,P.cream,5,4+bob,4,4);box(c,P.cream,16,5+bob,3,4)}else if(skin==="storm"){box(c,P.violet,16,3+bob,2,7);box(c,P.yellow,17,1+bob,2,4)}else{box(c,P.green,4,4+bob,5,4);box(c,P.pink,17,6+bob,2,2)}if(direction!=="up"){box(c,P.deep,9,10+bob,2,2);box(c,P.deep,14,10+bob,2,2)}if(direction==="left")box(c,accent,4,13+bob,3,3);if(direction==="right")box(c,accent,18,13+bob,3,3)}
function drawEnemy(c:Ctx,kind:EnemyKind,frame:number){const b=frame?1:0;if(kind==="mireling"){box(c,P.deep,2,7+b,16,12);box(c,"#3e7a46",3,6+b,14,11);box(c,P.green,5,5+b,10,8);box(c,P.white,6,9+b,3,2);box(c,P.white,12,9+b,3,2)}else if(kind==="midge"){box(c,P.deep,8,6+b,5,10);box(c,P.violet,9,7+b,3,8);box(c,"#9bd7c5",1,5+b,7,5);box(c,"#9bd7c5",13,5+b,7,5);box(c,P.yellow,9,5+b,3,2)}else if(kind==="spitter"){box(c,P.deep,2,5+b,18,14);box(c,P.teal,3,6+b,16,11);box(c,"#266c62",5,4+b,4,5);box(c,"#266c62",13,4+b,4,5);box(c,P.pink,8,12+b,6,3)}else if(kind==="brute"){box(c,P.deep,1,5+b,22,18);box(c,"#6f5a42",2,6+b,20,15);box(c,P.wood,5,3+b,5,7);box(c,P.wood,15,3+b,5,7);box(c,P.orange,8,11+b,8,4)}else{box(c,P.deep,3,8+b,42,39);box(c,"#254c35",4,7+b,40,37);box(c,"#387247",7,4+b,12,14);box(c,"#387247",29,4+b,12,14);box(c,P.green,10,14+b,28,20);box(c,P.yellow,13,19+b,6,4);box(c,P.yellow,29,19+b,6,4);box(c,P.deep,18,28+b,12,6);box(c,P.pink,21,29+b,6,3)}}

export function createPixelArt(scene:Phaser.Scene){
  for(let i=0;i<3;i++)make(scene,`ground-${i}`,16,16,c=>drawGround(c,i));
  make(scene,"sand",16,16,c=>{box(c,P.sand,0,0,16,16);box(c,P.sandLight,3,4,2,1);box(c,"#b77c45",12,11,1,1)});
  make(scene,"water",16,16,c=>{box(c,P.water,0,0,16,16);box(c,P.waterLight,2,4,6,1);box(c,"#237477",10,11,5,2)});
  make(scene,"flower",12,12,c=>{box(c,P.grassDark,5,5,2,7);box(c,P.pink,2,2,4,4);box(c,P.yellow,5,4,3,3);box(c,P.pink,7,1,3,4)});
  make(scene,"palm",24,32,c=>{box(c,P.deep,10,11,6,20);box(c,P.wood,11,10,4,20);box(c,P.grassDark,2,3,20,9);box(c,P.green,0,6,10,5);box(c,P.green,14,5,10,5);box(c,P.grassLight,8,1,9,8)});
  make(scene,"rock",16,12,c=>{box(c,P.deep,1,7,14,5);box(c,P.stone,2,4,12,6);box(c,"#94a088",5,3,6,2)});
  make(scene,"shell",10,9,c=>{box(c,P.deep,1,4,8,5);box(c,P.cream,2,2,6,5);box(c,P.pink,4,1,2,6)});
  make(scene,"ruin",24,24,c=>{box(c,P.deep,2,8,20,15);box(c,P.stone,3,5,18,17);box(c,"#94a088",5,4,5,4);box(c,"#4b5f54",12,10,7,3);box(c,P.grassDark,4,17,16,5)});
  for(const skin of ["sun","tide","storm","grove"] as Skin[])for(const direction of ["down","up","left","right"])for(let frame=0;frame<2;frame++)make(scene,`player-${skin}-${direction}-${frame}`,24,24,c=>drawPlayer(c,skin,direction,frame));
  const enemySizes:Record<EnemyKind,[number,number]>={mireling:[20,20],midge:[20,18],spitter:[22,22],brute:[24,26],colossus:[48,52]};
  for(const kind of Object.keys(enemySizes) as EnemyKind[])for(let frame=0;frame<2;frame++){const [w,h]=enemySizes[kind];make(scene,`enemy-${kind}-${frame}`,w,h,c=>drawEnemy(c,kind,frame))}
  for(const element of Object.keys(elementColors) as Element[])make(scene,`projectile-${element}`,8,8,c=>{box(c,P.deep,1,1,6,6);box(c,elementColors[element],2,2,4,4);box(c,P.white,3,2,2,2)});
  make(scene,"projectile-hostile",8,8,c=>{box(c,P.deep,0,2,8,4);box(c,P.red,2,1,4,6);box(c,P.yellow,3,2,2,3)});
  make(scene,"pickup-xp",8,8,c=>{box(c,P.deep,1,2,6,5);box(c,P.teal,2,1,4,5);box(c,P.white,3,2,2,2)});
  make(scene,"pickup-rune",12,12,c=>{box(c,P.deep,1,2,10,9);box(c,P.violet,2,1,8,9);box(c,P.white,5,3,2,5);box(c,P.yellow,3,5,6,2)});
  make(scene,"shadow",18,6,c=>{box(c,"rgba(8,18,14,.45)",1,1,16,4);box(c,"rgba(8,18,14,.25)",0,2,18,2)});
}
