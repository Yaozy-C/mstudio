// Three.js license: ./three.LICENSE.txt
import * as THREE from "./three.module.js";
const canvas = document.getElementById("screen");
const channel = location.pathname.split("/").filter(Boolean)[0];
let shot,
  renderer,
  manifest,
  playing = false,
  time = 0,
  last = performance.now();
const send = (type, extra = {}, transfer = []) =>
  parent.postMessage({ type, channel, ...extra }, "*", transfer);
function fail(error) {
  playing = false;
  const message = error?.message || String(error);
  document.getElementById("error").textContent = message;
  send("error", { message });
}
window.addEventListener("error", (e) => fail(e.error || e.message));
window.addEventListener("unhandledrejection", (e) => fail(e.reason));
function draw(t) {
  time = Math.max(
    0,
    Math.min(t, Math.max(0, manifest.target.duration - Number.EPSILON)),
  );
  shot.update?.(time);
  if (!shot.camera?.isCamera || !shot.scene?.isScene)
    throw Error("createShot 须返回 Three.js scene 与 camera");
  if (shot.camera.isPerspectiveCamera)
    shot.camera.aspect = manifest.target.width / manifest.target.height;
  shot.camera.updateProjectionMatrix?.();
  shot.scene.updateMatrixWorld(true);
  renderer.render(shot.scene, shot.camera);
}
async function init() {
  manifest = await (await fetch("manifest.json")).json();
  const factories = new Map();
  for (const model of manifest.models) {
    const module = await import(`./model-${model.id}.js`);
    if (typeof module.createModel !== "function")
      throw Error(`模型「${model.name}」缺少 createModel`);
    factories.set(model.id, module.createModel);
    factories.set(model.name, module.createModel);
  }
  for (const [alias, id] of Object.entries(manifest.target.bindings)) {
    if (factories.has(id)) factories.set(alias, factories.get(id));
  }
  const models = Object.freeze({
    create(id) {
      const make = factories.get(id);
      if (!make) throw Error(`镜头未声明共用模型：${id}`);
      const root = make({ THREE });
      if (!root?.isObject3D)
        throw Error(`模型「${id}」须返回 THREE.Group / Object3D`);
      return root;
    },
  });
  renderer = new THREE.WebGLRenderer({
    canvas,
    antialias: true,
    preserveDrawingBuffer: true,
  });
  renderer.setPixelRatio(1);
  renderer.setSize(manifest.target.width, manifest.target.height, false);
  renderer.shadowMap.enabled = true;
  renderer.shadowMap.type = THREE.PCFSoftShadowMap;
  renderer.outputColorSpace = THREE.SRGBColorSpace;
  renderer.toneMapping = THREE.ACESFilmicToneMapping;
  if (manifest.target.kind === "model") {
    const root = models.create(manifest.target.id);
    const scene = new THREE.Scene();
    scene.background = new THREE.Color("#262c32");
    scene.add(root);
    scene.add(new THREE.HemisphereLight(0xffffff, 0x718098, 2.5));
    const light = new THREE.DirectionalLight(0xffffff, 3);
    light.position.set(3, 5, 4);
    scene.add(light);
    const bounds = new THREE.Box3().setFromObject(root),
      center = bounds.getCenter(new THREE.Vector3());
    const extent = Math.max(bounds.getSize(new THREE.Vector3()).length(), 0.1);
    const camera = new THREE.PerspectiveCamera(38, 1, 0.001, extent * 100);
    shot = {
      scene,
      camera,
      update(t) {
        const a = (t / manifest.target.duration) * Math.PI * 2;
        camera.position.set(
          center.x + Math.sin(a + 0.6) * extent * 1.5,
          center.y + extent * 0.6,
          center.z + Math.cos(a + 0.6) * extent * 1.5,
        );
        camera.lookAt(center);
      },
    };
  } else {
    const module = await import("./shot.js");
    if (typeof module.createShot !== "function")
      throw Error("镜头文件缺少 createShot");
    shot = await module.createShot({ THREE, models, seed: 1337 });
  }
  draw(0);
  send("ready", {
    duration: manifest.target.duration,
    signature: manifest.signature,
  });
  function tick(now) {
    const dt = Math.min((now - last) / 1000, 0.1);
    last = now;
    if (playing) {
      try {
        draw((time + dt) % manifest.target.duration);
        send("time", { time });
      } catch (e) {
        fail(e);
      }
    }
    requestAnimationFrame(tick);
  }
  requestAnimationFrame(tick);
}
window.addEventListener("message", async (e) => {
  if (e.source !== parent || e.data?.channel !== channel || !shot) return;
  const { type, time: t, requestId } = e.data;
  try {
    if (type === "play") playing = !!e.data.playing;
    if (type === "seek") {
      playing = false;
      draw(Number(t));
      send("time", { time });
    }
    if (type === "frame") {
      playing = false;
      draw(Number(t));
      const blob = await new Promise((resolve) =>
        canvas.toBlob(resolve, "image/png"),
      );
      if (!blob) throw Error("无法读取 WebGL 画面");
      const bytes = await blob.arrayBuffer();
      send("frame", { requestId, bytes }, [bytes]);
    }
    if (type === "dispose") {
      playing = false;
      shot.dispose?.();
      renderer.dispose();
      shot = null;
    }
  } catch (e) {
    send("frame-error", { requestId, message: e.message });
    fail(e);
  }
});
init().catch(fail);
