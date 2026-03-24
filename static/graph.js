(function(){
var g=document.getElementById("graph");
var canvas=document.createElement("canvas");
canvas.width=g.clientWidth*2;canvas.height=g.clientHeight*2;
canvas.style.width="100%";canvas.style.height="100%";
g.appendChild(canvas);
var ctx=canvas.getContext("2d");
var W=canvas.width,H=canvas.height;
var nodes=[],edges=[],nodeMap={};
var ox=0,oy=0,scale=1,drag=null,dragNode=null;

fetch("/api/graph").then(function(r){return r.json()}).then(function(data){
  nodes=data.nodes;edges=data.edges;
  var n=nodes.length;
  for(var i=0;i<n;i++){
    var a=2*Math.PI*i/n;
    nodes[i].x=W/2+W*0.35*Math.cos(a);
    nodes[i].y=H/2+H*0.35*Math.sin(a);
    nodes[i].vx=0;nodes[i].vy=0;
    nodeMap[nodes[i].id]=nodes[i];
  }
  simulate();
});

function simulate(){
  for(var iter=0;iter<300;iter++){
    for(var i=0;i<nodes.length;i++){
      nodes[i].fx=0;nodes[i].fy=0;
      for(var j=0;j<nodes.length;j++){
        if(i===j)continue;
        var dx=nodes[i].x-nodes[j].x,dy=nodes[i].y-nodes[j].y;
        var d=Math.sqrt(dx*dx+dy*dy)+1;
        var f=50000/(d*d);
        nodes[i].fx+=f*dx/d;nodes[i].fy+=f*dy/d;
      }
    }
    for(var e=0;e<edges.length;e++){
      var s=nodeMap[edges[e].from],t=nodeMap[edges[e].to];
      if(!s||!t)continue;
      var dx=t.x-s.x,dy=t.y-s.y;
      var d=Math.sqrt(dx*dx+dy*dy)+1;
      var f=(d-200)*0.05;
      s.fx+=f*dx/d;s.fy+=f*dy/d;
      t.fx-=f*dx/d;t.fy-=f*dy/d;
    }
    for(var i=0;i<nodes.length;i++){
      nodes[i].fx+=(W/2-nodes[i].x)*0.01;
      nodes[i].fy+=(H/2-nodes[i].y)*0.01;
      nodes[i].vx=(nodes[i].vx+nodes[i].fx)*0.5;
      nodes[i].vy=(nodes[i].vy+nodes[i].fy)*0.5;
      nodes[i].x+=nodes[i].vx;nodes[i].y+=nodes[i].vy;
    }
  }
  draw();
}

function draw(){
  ctx.setTransform(1,0,0,1,0,0);
  ctx.clearRect(0,0,W,H);
  ctx.setTransform(scale,0,0,scale,ox,oy);

  // Edges with arrows
  for(var e=0;e<edges.length;e++){
    var s=nodeMap[edges[e].from],t=nodeMap[edges[e].to];
    if(!s||!t)continue;
    var tp=edges[e].type;
    var color=tp==="required"?"#dc2626":tp==="recommended"?"#d97706":"#16a34a";
    ctx.strokeStyle=color;ctx.lineWidth=2;
    ctx.beginPath();ctx.moveTo(s.x,s.y);ctx.lineTo(t.x,t.y);ctx.stroke();
    // Arrow
    var dx=t.x-s.x,dy=t.y-s.y,d=Math.sqrt(dx*dx+dy*dy);
    if(d<1)continue;
    var mx=t.x-dx/d*30,my=t.y-dy/d*30;
    var ax=-dy/d*8,ay=dx/d*8;
    ctx.fillStyle=color;ctx.beginPath();
    ctx.moveTo(t.x-dx/d*20,t.y-dy/d*20);
    ctx.lineTo(mx+ax,my+ay);ctx.lineTo(mx-ax,my-ay);ctx.fill();
  }
  // Nodes
  for(var i=0;i<nodes.length;i++){
    var n=nodes[i];
    ctx.beginPath();ctx.arc(n.x,n.y,20,0,2*Math.PI);
    ctx.fillStyle=n.lit?"#22c55e":"#eff6ff";
    ctx.fill();
    ctx.strokeStyle=n.lit?"#16a34a":"#2563eb";ctx.lineWidth=2;ctx.stroke();
    ctx.fillStyle="#1a1a1a";ctx.font="22px system-ui";ctx.textAlign="center";ctx.textBaseline="middle";
    ctx.fillText(n.name,n.x,n.y+36);
  }
}

// Pan & zoom
canvas.addEventListener("wheel",function(e){
  e.preventDefault();
  var r=e.deltaY>0?0.9:1.1;
  var rect=canvas.getBoundingClientRect();
  var mx=(e.clientX-rect.left)*2,my=(e.clientY-rect.top)*2;
  ox=mx-(mx-ox)*r;oy=my-(my-oy)*r;
  scale*=r;draw();
});
canvas.addEventListener("mousedown",function(e){
  var rect=canvas.getBoundingClientRect();
  var mx=((e.clientX-rect.left)*2-ox)/scale,my=((e.clientY-rect.top)*2-oy)/scale;
  for(var i=0;i<nodes.length;i++){
    var dx=nodes[i].x-mx,dy=nodes[i].y-my;
    if(dx*dx+dy*dy<900){dragNode=nodes[i];drag={x:e.clientX,y:e.clientY,nx:nodes[i].x,ny:nodes[i].y};return;}
  }
  drag={x:e.clientX,y:e.clientY,ox:ox,oy:oy};dragNode=null;
});
canvas.addEventListener("mousemove",function(e){
  if(!drag)return;
  if(dragNode){
    dragNode.x=drag.nx+(e.clientX-drag.x)*2/scale;
    dragNode.y=drag.ny+(e.clientY-drag.y)*2/scale;
    draw();
  }else{
    ox=drag.ox+(e.clientX-drag.x)*2;
    oy=drag.oy+(e.clientY-drag.y)*2;
    draw();
  }
});
canvas.addEventListener("mouseup",function(){drag=null;dragNode=null;});
canvas.addEventListener("mouseleave",function(){drag=null;dragNode=null;});
})();
