const IS_DEV = false;
const equalFn = (a, b) => a === b;
const $TRACK = /* @__PURE__ */ Symbol("solid-track");
const signalOptions = {
  equals: equalFn
};
let runEffects = runQueue;
const STALE = 1;
const PENDING = 2;
const UNOWNED = {
  owned: null,
  cleanups: null,
  context: null,
  owner: null
};
var Owner = null;
let Transition = null;
let ExternalSourceConfig = null;
let Listener = null;
let Updates = null;
let Effects = null;
let ExecCount = 0;
function createRoot(fn, detachedOwner) {
  const listener = Listener, owner = Owner, unowned = fn.length === 0, current = detachedOwner === void 0 ? owner : detachedOwner, root = unowned ? UNOWNED : {
    owned: null,
    cleanups: null,
    context: current ? current.context : null,
    owner: current
  }, updateFn = unowned ? fn : () => fn(() => untrack(() => cleanNode(root)));
  Owner = root;
  Listener = null;
  try {
    return runUpdates(updateFn, true);
  } finally {
    Listener = listener;
    Owner = owner;
  }
}
function createSignal(value, options) {
  options = options ? Object.assign({}, signalOptions, options) : signalOptions;
  const s = {
    value,
    observers: null,
    observerSlots: null,
    comparator: options.equals || void 0
  };
  const setter = (value2) => {
    if (typeof value2 === "function") {
      value2 = value2(s.value);
    }
    return writeSignal(s, value2);
  };
  return [readSignal.bind(s), setter];
}
function createRenderEffect(fn, value, options) {
  const c = createComputation(fn, value, false, STALE);
  updateComputation(c);
}
function createMemo(fn, value, options) {
  options = options ? Object.assign({}, signalOptions, options) : signalOptions;
  const c = createComputation(fn, value, true, 0);
  c.observers = null;
  c.observerSlots = null;
  c.comparator = options.equals || void 0;
  updateComputation(c);
  return readSignal.bind(c);
}
function untrack(fn) {
  if (Listener === null) return fn();
  const listener = Listener;
  Listener = null;
  try {
    if (ExternalSourceConfig) ;
    return fn();
  } finally {
    Listener = listener;
  }
}
function onCleanup(fn) {
  if (Owner === null) ;
  else if (Owner.cleanups === null) Owner.cleanups = [fn];
  else Owner.cleanups.push(fn);
  return fn;
}
function readSignal() {
  if (this.sources && this.state) {
    if (this.state === STALE) updateComputation(this);
    else {
      const updates = Updates;
      Updates = null;
      runUpdates(() => lookUpstream(this), false);
      Updates = updates;
    }
  }
  if (Listener) {
    const sSlot = this.observers ? this.observers.length : 0;
    if (!Listener.sources) {
      Listener.sources = [this];
      Listener.sourceSlots = [sSlot];
    } else {
      Listener.sources.push(this);
      Listener.sourceSlots.push(sSlot);
    }
    if (!this.observers) {
      this.observers = [Listener];
      this.observerSlots = [Listener.sources.length - 1];
    } else {
      this.observers.push(Listener);
      this.observerSlots.push(Listener.sources.length - 1);
    }
  }
  return this.value;
}
function writeSignal(node, value, isComp) {
  let current = node.value;
  if (!node.comparator || !node.comparator(current, value)) {
    node.value = value;
    if (node.observers && node.observers.length) {
      runUpdates(() => {
        for (let i = 0; i < node.observers.length; i += 1) {
          const o = node.observers[i];
          const TransitionRunning = Transition && Transition.running;
          if (TransitionRunning && Transition.disposed.has(o)) ;
          if (TransitionRunning ? !o.tState : !o.state) {
            if (o.pure) Updates.push(o);
            else Effects.push(o);
            if (o.observers) markDownstream(o);
          }
          if (!TransitionRunning) o.state = STALE;
        }
        if (Updates.length > 1e6) {
          Updates = [];
          if (IS_DEV) ;
          throw new Error();
        }
      }, false);
    }
  }
  return value;
}
function updateComputation(node) {
  if (!node.fn) return;
  cleanNode(node);
  const time = ExecCount;
  runComputation(node, node.value, time);
}
function runComputation(node, value, time) {
  let nextValue;
  const owner = Owner, listener = Listener;
  Listener = Owner = node;
  try {
    nextValue = node.fn(value);
  } catch (err) {
    if (node.pure) {
      {
        node.state = STALE;
        node.owned && node.owned.forEach(cleanNode);
        node.owned = null;
      }
    }
    node.updatedAt = time + 1;
    return handleError(err);
  } finally {
    Listener = listener;
    Owner = owner;
  }
  if (!node.updatedAt || node.updatedAt <= time) {
    if (node.updatedAt != null && "observers" in node) {
      writeSignal(node, nextValue);
    } else node.value = nextValue;
    node.updatedAt = time;
  }
}
function createComputation(fn, init, pure, state2 = STALE, options) {
  const c = {
    fn,
    state: state2,
    updatedAt: null,
    owned: null,
    sources: null,
    sourceSlots: null,
    cleanups: null,
    value: init,
    owner: Owner,
    context: Owner ? Owner.context : null,
    pure
  };
  if (Owner === null) ;
  else if (Owner !== UNOWNED) {
    {
      if (!Owner.owned) Owner.owned = [c];
      else Owner.owned.push(c);
    }
  }
  return c;
}
function runTop(node) {
  if (node.state === 0) return;
  if (node.state === PENDING) return lookUpstream(node);
  if (node.suspense && untrack(node.suspense.inFallback)) return node.suspense.effects.push(node);
  const ancestors = [node];
  while ((node = node.owner) && (!node.updatedAt || node.updatedAt < ExecCount)) {
    if (node.state) ancestors.push(node);
  }
  for (let i = ancestors.length - 1; i >= 0; i--) {
    node = ancestors[i];
    if (node.state === STALE) {
      updateComputation(node);
    } else if (node.state === PENDING) {
      const updates = Updates;
      Updates = null;
      runUpdates(() => lookUpstream(node, ancestors[0]), false);
      Updates = updates;
    }
  }
}
function runUpdates(fn, init) {
  if (Updates) return fn();
  let wait = false;
  if (!init) Updates = [];
  if (Effects) wait = true;
  else Effects = [];
  ExecCount++;
  try {
    const res = fn();
    completeUpdates(wait);
    return res;
  } catch (err) {
    if (!wait) Effects = null;
    Updates = null;
    handleError(err);
  }
}
function completeUpdates(wait) {
  if (Updates) {
    runQueue(Updates);
    Updates = null;
  }
  if (wait) return;
  const e = Effects;
  Effects = null;
  if (e.length) runUpdates(() => runEffects(e), false);
}
function runQueue(queue) {
  for (let i = 0; i < queue.length; i++) runTop(queue[i]);
}
function lookUpstream(node, ignore) {
  node.state = 0;
  for (let i = 0; i < node.sources.length; i += 1) {
    const source = node.sources[i];
    if (source.sources) {
      const state2 = source.state;
      if (state2 === STALE) {
        if (source !== ignore && (!source.updatedAt || source.updatedAt < ExecCount)) runTop(source);
      } else if (state2 === PENDING) lookUpstream(source, ignore);
    }
  }
}
function markDownstream(node) {
  for (let i = 0; i < node.observers.length; i += 1) {
    const o = node.observers[i];
    if (!o.state) {
      o.state = PENDING;
      if (o.pure) Updates.push(o);
      else Effects.push(o);
      o.observers && markDownstream(o);
    }
  }
}
function cleanNode(node) {
  let i;
  if (node.sources) {
    while (node.sources.length) {
      const source = node.sources.pop(), index = node.sourceSlots.pop(), obs = source.observers;
      if (obs && obs.length) {
        const n = obs.pop(), s = source.observerSlots.pop();
        if (index < obs.length) {
          n.sourceSlots[s] = index;
          obs[index] = n;
          source.observerSlots[index] = s;
        }
      }
    }
  }
  if (node.tOwned) {
    for (i = node.tOwned.length - 1; i >= 0; i--) cleanNode(node.tOwned[i]);
    delete node.tOwned;
  }
  if (node.owned) {
    for (i = node.owned.length - 1; i >= 0; i--) cleanNode(node.owned[i]);
    node.owned = null;
  }
  if (node.cleanups) {
    for (i = node.cleanups.length - 1; i >= 0; i--) node.cleanups[i]();
    node.cleanups = null;
  }
  node.state = 0;
}
function castError(err) {
  if (err instanceof Error) return err;
  return new Error(typeof err === "string" ? err : "Unknown error", {
    cause: err
  });
}
function handleError(err, owner = Owner) {
  const error = castError(err);
  throw error;
}
const FALLBACK = /* @__PURE__ */ Symbol("fallback");
function dispose$1(d) {
  for (let i = 0; i < d.length; i++) d[i]();
}
function mapArray(list, mapFn, options = {}) {
  let items = [], mapped = [], disposers = [], len = 0, indexes = mapFn.length > 1 ? [] : null;
  onCleanup(() => dispose$1(disposers));
  return () => {
    let newItems = list() || [], newLen = newItems.length, i, j;
    newItems[$TRACK];
    return untrack(() => {
      let newIndices, newIndicesNext, temp, tempdisposers, tempIndexes, start, end, newEnd, item;
      if (newLen === 0) {
        if (len !== 0) {
          dispose$1(disposers);
          disposers = [];
          items = [];
          mapped = [];
          len = 0;
          indexes && (indexes = []);
        }
        if (options.fallback) {
          items = [FALLBACK];
          mapped[0] = createRoot((disposer) => {
            disposers[0] = disposer;
            return options.fallback();
          });
          len = 1;
        }
      } else if (len === 0) {
        mapped = new Array(newLen);
        for (j = 0; j < newLen; j++) {
          items[j] = newItems[j];
          mapped[j] = createRoot(mapper);
        }
        len = newLen;
      } else {
        temp = new Array(newLen);
        tempdisposers = new Array(newLen);
        indexes && (tempIndexes = new Array(newLen));
        for (start = 0, end = Math.min(len, newLen); start < end && items[start] === newItems[start]; start++) ;
        for (end = len - 1, newEnd = newLen - 1; end >= start && newEnd >= start && items[end] === newItems[newEnd]; end--, newEnd--) {
          temp[newEnd] = mapped[end];
          tempdisposers[newEnd] = disposers[end];
          indexes && (tempIndexes[newEnd] = indexes[end]);
        }
        newIndices = /* @__PURE__ */ new Map();
        newIndicesNext = new Array(newEnd + 1);
        for (j = newEnd; j >= start; j--) {
          item = newItems[j];
          i = newIndices.get(item);
          newIndicesNext[j] = i === void 0 ? -1 : i;
          newIndices.set(item, j);
        }
        for (i = start; i <= end; i++) {
          item = items[i];
          j = newIndices.get(item);
          if (j !== void 0 && j !== -1) {
            temp[j] = mapped[i];
            tempdisposers[j] = disposers[i];
            indexes && (tempIndexes[j] = indexes[i]);
            j = newIndicesNext[j];
            newIndices.set(item, j);
          } else disposers[i]();
        }
        for (j = start; j < newLen; j++) {
          if (j in temp) {
            mapped[j] = temp[j];
            disposers[j] = tempdisposers[j];
            if (indexes) {
              indexes[j] = tempIndexes[j];
              indexes[j](j);
            }
          } else mapped[j] = createRoot(mapper);
        }
        mapped = mapped.slice(0, len = newLen);
        items = newItems.slice(0);
      }
      return mapped;
    });
    function mapper(disposer) {
      disposers[j] = disposer;
      if (indexes) {
        const [s, set] = createSignal(j);
        indexes[j] = set;
        return mapFn(newItems[j], s);
      }
      return mapFn(newItems[j]);
    }
  };
}
function createComponent(Comp, props) {
  return untrack(() => Comp(props || {}));
}
const narrowedError = (name) => `Stale read from <${name}>.`;
function For(props) {
  const fallback = "fallback" in props && {
    fallback: () => props.fallback
  };
  return createMemo(mapArray(() => props.each, props.children, fallback || void 0));
}
function Show(props) {
  const keyed = props.keyed;
  const conditionValue = createMemo(() => props.when, void 0, void 0);
  const condition = keyed ? conditionValue : createMemo(conditionValue, void 0, {
    equals: (a, b) => !a === !b
  });
  return createMemo(() => {
    const c = condition();
    if (c) {
      const child = props.children;
      const fn = typeof child === "function" && child.length > 0;
      return fn ? untrack(() => child(keyed ? c : () => {
        if (!untrack(condition)) throw narrowedError("Show");
        return conditionValue();
      })) : child;
    }
    return props.fallback;
  }, void 0, void 0);
}
const memo = (fn) => createMemo(() => fn());
function reconcileArrays(parentNode, a, b) {
  let bLength = b.length, aEnd = a.length, bEnd = bLength, aStart = 0, bStart = 0, after = a[aEnd - 1].nextSibling, map = null;
  while (aStart < aEnd || bStart < bEnd) {
    if (a[aStart] === b[bStart]) {
      aStart++;
      bStart++;
      continue;
    }
    while (a[aEnd - 1] === b[bEnd - 1]) {
      aEnd--;
      bEnd--;
    }
    if (aEnd === aStart) {
      const node = bEnd < bLength ? bStart ? b[bStart - 1].nextSibling : b[bEnd - bStart] : after;
      while (bStart < bEnd) parentNode.insertBefore(b[bStart++], node);
    } else if (bEnd === bStart) {
      while (aStart < aEnd) {
        if (!map || !map.has(a[aStart])) a[aStart].remove();
        aStart++;
      }
    } else if (a[aStart] === b[bEnd - 1] && b[bStart] === a[aEnd - 1]) {
      const node = a[--aEnd].nextSibling;
      parentNode.insertBefore(b[bStart++], a[aStart++].nextSibling);
      parentNode.insertBefore(b[--bEnd], node);
      a[aEnd] = b[bEnd];
    } else {
      if (!map) {
        map = /* @__PURE__ */ new Map();
        let i = bStart;
        while (i < bEnd) map.set(b[i], i++);
      }
      const index = map.get(a[aStart]);
      if (index != null) {
        if (bStart < index && index < bEnd) {
          let i = aStart, sequence = 1, t;
          while (++i < aEnd && i < bEnd) {
            if ((t = map.get(a[i])) == null || t !== index + sequence) break;
            sequence++;
          }
          if (sequence > index - bStart) {
            const node = a[aStart];
            while (bStart < index) parentNode.insertBefore(b[bStart++], node);
          } else parentNode.replaceChild(b[bStart++], a[aStart++]);
        } else aStart++;
      } else a[aStart++].remove();
    }
  }
}
const $$EVENTS = "_$DX_DELEGATE";
function render$1(code, element, init, options = {}) {
  let disposer;
  createRoot((dispose2) => {
    disposer = dispose2;
    element === document ? code() : insert(element, code(), element.firstChild ? null : void 0, init);
  }, options.owner);
  return () => {
    disposer();
    element.textContent = "";
  };
}
function template(html, isImportNode, isSVG, isMathML) {
  let node;
  const create = () => {
    const t = document.createElement("template");
    t.innerHTML = html;
    return t.content.firstChild;
  };
  const fn = () => (node || (node = create())).cloneNode(true);
  fn.cloneNode = fn;
  return fn;
}
function delegateEvents(eventNames, document2 = window.document) {
  const e = document2[$$EVENTS] || (document2[$$EVENTS] = /* @__PURE__ */ new Set());
  for (let i = 0, l = eventNames.length; i < l; i++) {
    const name = eventNames[i];
    if (!e.has(name)) {
      e.add(name);
      document2.addEventListener(name, eventHandler);
    }
  }
}
function setAttribute(node, name, value) {
  if (value == null) node.removeAttribute(name);
  else node.setAttribute(name, value);
}
function className(node, value) {
  if (value == null) node.removeAttribute("class");
  else node.className = value;
}
function style(node, value, prev) {
  if (!value) return prev ? setAttribute(node, "style") : value;
  const nodeStyle = node.style;
  if (typeof value === "string") return nodeStyle.cssText = value;
  typeof prev === "string" && (nodeStyle.cssText = prev = void 0);
  prev || (prev = {});
  value || (value = {});
  let v, s;
  for (s in prev) {
    value[s] == null && nodeStyle.removeProperty(s);
    delete prev[s];
  }
  for (s in value) {
    v = value[s];
    if (v !== prev[s]) {
      nodeStyle.setProperty(s, v);
      prev[s] = v;
    }
  }
  return prev;
}
function insert(parent, accessor, marker, initial) {
  if (marker !== void 0 && !initial) initial = [];
  if (typeof accessor !== "function") return insertExpression(parent, accessor, initial, marker);
  createRenderEffect((current) => insertExpression(parent, accessor(), current, marker), initial);
}
function eventHandler(e) {
  let node = e.target;
  const key = `$$${e.type}`;
  const oriTarget = e.target;
  const oriCurrentTarget = e.currentTarget;
  const retarget = (value) => Object.defineProperty(e, "target", {
    configurable: true,
    value
  });
  const handleNode = () => {
    const handler = node[key];
    if (handler && !node.disabled) {
      const data = node[`${key}Data`];
      data !== void 0 ? handler.call(node, data, e) : handler.call(node, e);
      if (e.cancelBubble) return;
    }
    node.host && typeof node.host !== "string" && !node.host._$host && node.contains(e.target) && retarget(node.host);
    return true;
  };
  const walkUpTree = () => {
    while (handleNode() && (node = node._$host || node.parentNode || node.host)) ;
  };
  Object.defineProperty(e, "currentTarget", {
    configurable: true,
    get() {
      return node || document;
    }
  });
  if (e.composedPath) {
    const path = e.composedPath();
    retarget(path[0]);
    for (let i = 0; i < path.length - 2; i++) {
      node = path[i];
      if (!handleNode()) break;
      if (node._$host) {
        node = node._$host;
        walkUpTree();
        break;
      }
      if (node.parentNode === oriCurrentTarget) {
        break;
      }
    }
  } else walkUpTree();
  retarget(oriTarget);
}
function insertExpression(parent, value, current, marker, unwrapArray) {
  while (typeof current === "function") current = current();
  if (value === current) return current;
  const t = typeof value, multi = marker !== void 0;
  parent = multi && current[0] && current[0].parentNode || parent;
  if (t === "string" || t === "number") {
    if (t === "number") {
      value = value.toString();
      if (value === current) return current;
    }
    if (multi) {
      let node = current[0];
      if (node && node.nodeType === 3) {
        node.data !== value && (node.data = value);
      } else node = document.createTextNode(value);
      current = cleanChildren(parent, current, marker, node);
    } else {
      if (current !== "" && typeof current === "string") {
        current = parent.firstChild.data = value;
      } else current = parent.textContent = value;
    }
  } else if (value == null || t === "boolean") {
    current = cleanChildren(parent, current, marker);
  } else if (t === "function") {
    createRenderEffect(() => {
      let v = value();
      while (typeof v === "function") v = v();
      current = insertExpression(parent, v, current, marker);
    });
    return () => current;
  } else if (Array.isArray(value)) {
    const array = [];
    const currentArray = current && Array.isArray(current);
    if (normalizeIncomingArray(array, value, current, unwrapArray)) {
      createRenderEffect(() => current = insertExpression(parent, array, current, marker, true));
      return () => current;
    }
    if (array.length === 0) {
      current = cleanChildren(parent, current, marker);
      if (multi) return current;
    } else if (currentArray) {
      if (current.length === 0) {
        appendNodes(parent, array, marker);
      } else reconcileArrays(parent, current, array);
    } else {
      current && cleanChildren(parent);
      appendNodes(parent, array);
    }
    current = array;
  } else if (value.nodeType) {
    if (Array.isArray(current)) {
      if (multi) return current = cleanChildren(parent, current, marker, value);
      cleanChildren(parent, current, null, value);
    } else if (current == null || current === "" || !parent.firstChild) {
      parent.appendChild(value);
    } else parent.replaceChild(value, parent.firstChild);
    current = value;
  } else ;
  return current;
}
function normalizeIncomingArray(normalized, array, current, unwrap) {
  let dynamic = false;
  for (let i = 0, len = array.length; i < len; i++) {
    let item = array[i], prev = current && current[normalized.length], t;
    if (item == null || item === true || item === false) ;
    else if ((t = typeof item) === "object" && item.nodeType) {
      normalized.push(item);
    } else if (Array.isArray(item)) {
      dynamic = normalizeIncomingArray(normalized, item, prev) || dynamic;
    } else if (t === "function") {
      if (unwrap) {
        while (typeof item === "function") item = item();
        dynamic = normalizeIncomingArray(normalized, Array.isArray(item) ? item : [item], Array.isArray(prev) ? prev : [prev]) || dynamic;
      } else {
        normalized.push(item);
        dynamic = true;
      }
    } else {
      const value = String(item);
      if (prev && prev.nodeType === 3 && prev.data === value) normalized.push(prev);
      else normalized.push(document.createTextNode(value));
    }
  }
  return dynamic;
}
function appendNodes(parent, array, marker = null) {
  for (let i = 0, len = array.length; i < len; i++) parent.insertBefore(array[i], marker);
}
function cleanChildren(parent, current, marker, replacement) {
  if (marker === void 0) return parent.textContent = "";
  const node = replacement || document.createTextNode("");
  if (current.length) {
    let inserted = false;
    for (let i = current.length - 1; i >= 0; i--) {
      const el = current[i];
      if (node !== el) {
        const isParent = el.parentNode === parent;
        if (!inserted && !i) isParent ? parent.replaceChild(node, el) : parent.insertBefore(node, marker);
        else isParent && el.remove();
      } else inserted = true;
    }
  } else parent.insertBefore(node, marker);
  return [node];
}
const TEST_API_GLOBAL_NAME = "__AW_TEST__";
const RENDER_META_GLOBAL_NAME = "__AW_VIEWER_RENDER_META__";
const SOFTWARE_SAFE_RENDER_MODE = "software_safe";
const VIEWER_AUTH_BOOTSTRAP_OBJECT = "__OASIS7_VIEWER_AUTH_ENV";
const VIEWER_PLAYER_ID_KEY = "OASIS7_VIEWER_PLAYER_ID";
const VIEWER_AUTH_PUBLIC_KEY = "OASIS7_VIEWER_AUTH_PUBLIC_KEY";
const VIEWER_AUTH_PRIVATE_KEY = "OASIS7_VIEWER_AUTH_PRIVATE_KEY";
const VIEWER_AUTH_SIGNATURE_PREFIX = "awviewauth:v1:";
const HOSTED_PLAYER_SESSION_STORAGE_PREFIX = "oasis7.hosted_player_session.v1";
const HOSTED_PLAYER_SESSION_ADMISSION_ROUTE = "/api/public/player-session/admission";
const HOSTED_PLAYER_SESSION_REFRESH_ROUTE = "/api/public/player-session/refresh";
const HOSTED_PLAYER_SESSION_ISSUE_ROUTE = "/api/public/player-session/issue";
const HOSTED_PLAYER_SESSION_RELEASE_ROUTE = "/api/public/player-session/release";
const HOSTED_STRONG_AUTH_GRANT_ROUTE = "/api/public/strong-auth/grant";
const HOSTED_PLAYER_SESSION_REFRESH_INTERVAL_MS = 3e4;
const DEFAULT_WS_ADDR = "ws://127.0.0.1:5011";
const MAX_EVENTS = 24;
const SOFTWARE_RENDERER_MARKERS = [
  "swiftshader",
  "llvmpipe",
  "software rasterizer",
  "basic render driver",
  "softpipe",
  "lavapipe"
];
const ED25519_PKCS8_PREFIX = new Uint8Array([
  48,
  46,
  2,
  1,
  0,
  48,
  5,
  6,
  3,
  43,
  101,
  112,
  4,
  34,
  4,
  32
]);
const textEncoder = new TextEncoder();
const state = {
  connectionStatus: "connecting",
  logicalTime: 0,
  eventSeq: 0,
  tick: 0,
  selectedKind: null,
  selectedId: null,
  errorCount: 0,
  lastError: null,
  eventCount: 0,
  traceCount: 0,
  cameraMode: "software_safe",
  cameraRadius: 0,
  cameraOrthoScale: 0,
  renderMode: SOFTWARE_SAFE_RENDER_MODE,
  rendererClass: "none",
  softwareSafeReason: null,
  renderer: null,
  vendor: null,
  webglVersion: null,
  controlProfile: "playback",
  debugViewerMode: "debug_viewer",
  debugViewerStatus: "detached",
  worldId: null,
  server: null,
  wsUrl: null,
  lastControlFeedback: null,
  lastPromptFeedback: null,
  lastChatFeedback: null,
  snapshot: null,
  metrics: null,
  hostedAccess: null,
  hostedAdmission: null,
  recentEvents: [],
  chatHistory: [],
  selectedObject: null,
  auth: {
    available: false,
    playerId: null,
    publicKey: null,
    privateKey: null,
    releaseToken: null,
    error: null,
    revokeReason: null,
    revokedBy: null,
    source: "guest_only",
    registrationStatus: "guest",
    sessionEpoch: null,
    issuedAtUnixMs: null,
    recoveryErrorCode: null,
    recoveryErrorMessage: null,
    issueInFlight: false,
    syncInFlight: false,
    runtimeStatus: "guest",
    boundAgentId: null,
    pendingRequestedAgentId: null,
    pendingForceRebind: false,
    rebindNotice: null
  },
  promptDraft: {
    agentId: null,
    currentVersion: 0,
    rollbackTargetVersion: 0,
    updatedBy: "",
    updatedAtTick: 0,
    systemPrompt: "",
    shortTermGoal: "",
    longTermGoal: "",
    dirty: false
  },
  chatDraft: {
    agentId: null,
    message: "",
    dirty: false
  },
  strongAuth: {
    approvalCode: "",
    lastGrantActionId: null,
    lastGrantExpiresAtUnixMs: null,
    lastGrantError: null
  }
};
let socket = null;
let reconnectTimer = null;
let hostedSessionRefreshTimer = null;
let requestId = 0;
let authNonceCounter = 0;
let selectedSearch = "";
let semanticSendLoop = null;
const pendingControlFeedback = /* @__PURE__ */ new Map();
const pendingSemanticCommands = [];
const authKeyCache = /* @__PURE__ */ new Map();
let pendingSessionRegisterWaiter = null;
let renderHook = () => {
};
let bootstrapped = false;
function getSelectedSearch() {
  return selectedSearch;
}
function setSelectedSearch(value) {
  selectedSearch = String(value || "");
  render();
}
function setRenderHook(nextHook) {
  renderHook = typeof nextHook === "function" ? nextHook : () => {
  };
}
function getSearchParams() {
  return new URLSearchParams(window.location.search || "");
}
function normalizeWsAddr(raw) {
  const value = String(raw || "").trim();
  if (!value) return DEFAULT_WS_ADDR;
  if (value.startsWith("ws://") || value.startsWith("wss://")) return value;
  if (value.startsWith("http://")) return `ws://${value.slice("http://".length)}`;
  if (value.startsWith("https://")) return `wss://${value.slice("https://".length)}`;
  return `ws://${value}`;
}
function clone(value) {
  return value == null ? value : JSON.parse(JSON.stringify(value));
}
function detectRendererMeta() {
  const params = getSearchParams();
  const reasonFromQuery = params.get("software_safe_reason");
  const meta = {
    renderMode: SOFTWARE_SAFE_RENDER_MODE,
    rendererClass: "none",
    softwareSafeReason: reasonFromQuery || "forced_fallback",
    renderer: null,
    vendor: null,
    webglVersion: null
  };
  try {
    const canvas = document.createElement("canvas");
    const gl = canvas.getContext("webgl") || canvas.getContext("experimental-webgl");
    if (!gl) {
      meta.rendererClass = "none";
      meta.softwareSafeReason = reasonFromQuery || "webgl_unavailable";
      return meta;
    }
    meta.webglVersion = gl.getParameter(gl.VERSION) || null;
    const debugInfo = gl.getExtension("WEBGL_debug_renderer_info");
    if (debugInfo) {
      meta.renderer = gl.getParameter(debugInfo.UNMASKED_RENDERER_WEBGL) || null;
      meta.vendor = gl.getParameter(debugInfo.UNMASKED_VENDOR_WEBGL) || null;
    }
    const rendererText = String(meta.renderer || "").toLowerCase();
    if (SOFTWARE_RENDERER_MARKERS.some((marker) => rendererText.includes(marker))) {
      meta.rendererClass = "software";
      meta.softwareSafeReason = reasonFromQuery || "software_renderer_detected";
    } else {
      meta.rendererClass = "unknown";
      meta.softwareSafeReason = reasonFromQuery || "forced_query";
    }
  } catch (error) {
    meta.rendererClass = "none";
    meta.softwareSafeReason = reasonFromQuery || "webgl_probe_failed";
    meta.renderer = String(error);
  }
  return meta;
}
function resolveAuthBootstrap() {
  const raw = window[VIEWER_AUTH_BOOTSTRAP_OBJECT];
  if (!raw || typeof raw !== "object") {
    return {
      available: false,
      playerId: null,
      publicKey: null,
      privateKey: null,
      releaseToken: null,
      error: "viewer auth bootstrap is unavailable",
      revokeReason: null,
      revokedBy: null,
      source: "guest_only",
      registrationStatus: "guest",
      sessionEpoch: null,
      issuedAtUnixMs: null,
      recoveryErrorCode: null,
      recoveryErrorMessage: null,
      issueInFlight: false,
      syncInFlight: false,
      runtimeStatus: "guest",
      boundAgentId: null,
      pendingRequestedAgentId: null,
      pendingForceRebind: false,
      rebindNotice: null
    };
  }
  const playerId = String(raw[VIEWER_PLAYER_ID_KEY] || "").trim();
  const publicKey = String(raw[VIEWER_AUTH_PUBLIC_KEY] || "").trim().toLowerCase();
  const privateKey = String(raw[VIEWER_AUTH_PRIVATE_KEY] || "").trim().toLowerCase();
  if (!playerId || !publicKey || !privateKey) {
    return {
      available: false,
      playerId: playerId || null,
      publicKey: publicKey || null,
      privateKey: privateKey || null,
      releaseToken: null,
      error: "viewer auth bootstrap is incomplete",
      revokeReason: null,
      revokedBy: null,
      source: "guest_only",
      registrationStatus: "guest",
      sessionEpoch: null,
      issuedAtUnixMs: null,
      recoveryErrorCode: null,
      recoveryErrorMessage: null,
      issueInFlight: false,
      syncInFlight: false,
      runtimeStatus: "guest",
      boundAgentId: null,
      pendingRequestedAgentId: null,
      pendingForceRebind: false,
      rebindNotice: null
    };
  }
  return {
    available: true,
    playerId,
    publicKey,
    privateKey,
    releaseToken: null,
    error: null,
    revokeReason: null,
    revokedBy: null,
    source: "legacy_viewer_auth_bootstrap",
    registrationStatus: "registered",
    sessionEpoch: 1,
    issuedAtUnixMs: null,
    recoveryErrorCode: null,
    recoveryErrorMessage: null,
    issueInFlight: false,
    syncInFlight: false,
    runtimeStatus: "legacy_preview",
    boundAgentId: null,
    pendingRequestedAgentId: null,
    pendingForceRebind: false,
    rebindNotice: null
  };
}
function initialWsUrl() {
  const params = getSearchParams();
  return normalizeWsAddr(params.get("ws") || params.get("addr") || DEFAULT_WS_ADDR);
}
function hostedPlayerSessionStorageKey() {
  return `${HOSTED_PLAYER_SESSION_STORAGE_PREFIX}:${initialWsUrl()}`;
}
function persistHostedPlayerSession(auth) {
  if (!auth?.available || !auth?.playerId || !auth?.publicKey || !auth?.privateKey || auth.source === "legacy_viewer_auth_bootstrap") {
    return;
  }
  try {
    window.localStorage?.setItem(
      hostedPlayerSessionStorageKey(),
      JSON.stringify({
        playerId: auth.playerId,
        publicKey: auth.publicKey,
        privateKey: auth.privateKey,
        releaseToken: auth.releaseToken || null,
        issuedAtUnixMs: auth.issuedAtUnixMs || null,
        sessionEpoch: auth.sessionEpoch || null
      })
    );
  } catch (_) {
  }
}
function clearHostedPlayerSession() {
  try {
    window.localStorage?.removeItem(hostedPlayerSessionStorageKey());
  } catch (_) {
  }
}
function resolveStoredHostedPlayerSession() {
  try {
    const raw = window.localStorage?.getItem(hostedPlayerSessionStorageKey());
    if (!raw) {
      return null;
    }
    const parsed = JSON.parse(raw);
    const playerId = String(parsed?.playerId || "").trim();
    const publicKey = String(parsed?.publicKey || "").trim().toLowerCase();
    const privateKey = String(parsed?.privateKey || "").trim().toLowerCase();
    const releaseToken = String(parsed?.releaseToken || "").trim();
    if (!playerId || !publicKey || !privateKey || !releaseToken) {
      clearHostedPlayerSession();
      return null;
    }
    return {
      available: true,
      playerId,
      publicKey,
      privateKey,
      releaseToken,
      error: null,
      revokeReason: null,
      revokedBy: null,
      source: "hosted_browser_storage",
      registrationStatus: "issued",
      sessionEpoch: parsed?.sessionEpoch == null ? null : Number(parsed.sessionEpoch),
      issuedAtUnixMs: parsed?.issuedAtUnixMs == null ? null : Number(parsed.issuedAtUnixMs),
      recoveryErrorCode: null,
      recoveryErrorMessage: null,
      issueInFlight: false,
      syncInFlight: false,
      runtimeStatus: "issued",
      boundAgentId: null,
      pendingRequestedAgentId: null,
      pendingForceRebind: false,
      rebindNotice: null
    };
  } catch (_) {
    clearHostedPlayerSession();
    return null;
  }
}
function resolveViewerAuthState() {
  const bootstrap2 = resolveAuthBootstrap();
  if (bootstrap2.available) {
    return bootstrap2;
  }
  return resolveStoredHostedPlayerSession() || bootstrap2;
}
async function refreshHostedAdmissionState() {
  if (String(state.hostedAccess?.deployment_mode || "").trim() !== "hosted_public_join") {
    state.hostedAdmission = null;
    return null;
  }
  try {
    const response = await fetch(HOSTED_PLAYER_SESSION_ADMISSION_ROUTE, {
      method: "GET",
      cache: "no-store",
      headers: { Accept: "application/json" }
    });
    const payload = await response.json();
    state.hostedAdmission = payload?.admission ? clone(payload.admission) : null;
    return state.hostedAdmission;
  } catch (_) {
    return state.hostedAdmission;
  }
}
async function refreshHostedPlayerLease() {
  const playerId = String(state.auth.playerId || "").trim();
  const releaseToken = String(state.auth.releaseToken || "").trim();
  if (!playerId || !releaseToken || state.auth.source === "legacy_viewer_auth_bootstrap") {
    return null;
  }
  try {
    const response = await fetch(
      `${HOSTED_PLAYER_SESSION_REFRESH_ROUTE}?player_id=${encodeURIComponent(playerId)}&release_token=${encodeURIComponent(releaseToken)}`,
      {
        method: "POST",
        cache: "no-store",
        headers: { Accept: "application/json" }
      }
    );
    const payload = await response.json();
    if (payload?.admission) {
      state.hostedAdmission = clone(payload.admission);
    }
    if (!response.ok || !payload?.ok) {
      throw new Error(payload?.error || payload?.error_code || `hosted player-session refresh failed with HTTP ${response.status}`);
    }
    return payload;
  } catch (error) {
    state.auth.error = String(error);
    return null;
  }
}
function stopHostedSessionRefreshLoop() {
  if (hostedSessionRefreshTimer) {
    window.clearInterval(hostedSessionRefreshTimer);
    hostedSessionRefreshTimer = null;
  }
}
function syncHostedSessionRefreshLoop() {
  const shouldRun = state.connectionStatus === "connected" && state.auth.available && state.auth.source !== "legacy_viewer_auth_bootstrap" && state.auth.registrationStatus === "registered" && !!state.auth.releaseToken;
  if (!shouldRun) {
    stopHostedSessionRefreshLoop();
    return;
  }
  if (hostedSessionRefreshTimer) {
    return;
  }
  hostedSessionRefreshTimer = window.setInterval(() => {
    probeHostedRuntimeSession();
    void refreshHostedPlayerLease().then(() => render());
  }, HOSTED_PLAYER_SESSION_REFRESH_INTERVAL_MS);
}
function resolveHostedAccessHint() {
  const raw = getSearchParams().get("hosted_access");
  if (!raw) {
    return null;
  }
  try {
    const parsed = JSON.parse(raw);
    return parsed && typeof parsed === "object" ? parsed : null;
  } catch (_) {
    return null;
  }
}
function hostnameFromUrl(raw) {
  const value = String(raw || "").trim();
  if (!value) return null;
  try {
    return new URL(value, window.location.href).hostname || null;
  } catch (_) {
    return null;
  }
}
function isLoopbackHostname(raw) {
  const value = String(raw || "").trim().toLowerCase();
  return value === "localhost" || value === "127.0.0.1" || value === "::1" || value === "[::1]";
}
function authDeploymentHint(auth) {
  const hostedMode = String(state.hostedAccess?.deployment_mode || "").trim();
  if (hostedMode === "hosted_public_join") {
    if (auth.available && auth.source === "legacy_viewer_auth_bootstrap") {
      return "hosted_public_join_contract_with_legacy_bootstrap";
    }
    return auth.available ? "hosted_public_join_contract_with_browser_session" : "hosted_public_join_contract";
  }
  if (hostedMode === "trusted_local_only") {
    return auth.available ? "trusted_local_contract" : "trusted_local_contract_guest";
  }
  const params = getSearchParams();
  const wsHost = hostnameFromUrl(state.wsUrl || params.get("ws") || params.get("addr") || "");
  const pageHost = String(window.location.hostname || "").trim();
  const remoteOriginLikely = [pageHost, wsHost].filter(Boolean).some((host) => !isLoopbackHostname(host));
  if (auth.available) {
    return remoteOriginLikely ? "remote_origin_legacy_bootstrap" : "trusted_local_preview";
  }
  return remoteOriginLikely ? "hosted_public_join_likely" : "guest_only_or_missing_bootstrap";
}
function isHostedPublicJoinHint(deploymentHint) {
  return [
    "hosted_public_join_contract",
    "hosted_public_join_contract_with_legacy_bootstrap",
    "hosted_public_join_likely"
  ].includes(deploymentHint);
}
function hostedActionPolicy(actionId) {
  const normalizedActionId = actionId === "prompt_control" ? "prompt_control_apply" : actionId;
  return state.hostedAccess?.action_matrix?.find((policy) => policy?.action_id === normalizedActionId) || null;
}
function guestSessionReason(auth, deploymentHint) {
  if (auth.available) {
    return auth.source === "legacy_viewer_auth_bootstrap" ? "guest session has already been superseded by the current preview player auth lane" : "guest session has already been superseded by a hosted-issued player identity";
  }
  if (isHostedPublicJoinHint(deploymentHint)) {
    return auth.error || "this browser is still guest-only; hosted public join must issue a player identity before low-risk interaction unlocks";
  }
  return auth.error || "viewer auth bootstrap is unavailable, so the browser cannot leave guest session";
}
function playerSessionReason(auth, deploymentHint) {
  if (auth.available) {
    if (auth.source === "legacy_viewer_auth_bootstrap") {
      return "player interaction is currently unlocked through legacy viewer auth bootstrap in trusted preview mode";
    }
    if (auth.registrationStatus === "registered") {
      return "player interaction is unlocked through hosted-issued player_id + browser-local ephemeral Ed25519 session";
    }
    if (auth.registrationStatus === "registering" || auth.registrationStatus === "issued") {
      return "browser-local hosted identity is ready; runtime player-session registration is still in progress";
    }
    return auth.error || "hosted player identity exists, but runtime registration still needs recovery";
  }
  if (isHostedPublicJoinHint(deploymentHint)) {
    return auth.error || "player session upgrade/login is still pending hosted issue";
  }
  return auth.error || "viewer auth bootstrap is missing or incomplete";
}
function strongAuthReason() {
  return "strong auth remains a separate upgrade plane; software_safe only previews backend reauth for prompt_control and still does not issue hosted-ready asset/governance proofs";
}
function isStrongAuthSensitiveAction(actionId) {
  const policy = hostedActionPolicy(actionId);
  if (policy) {
    return policy.required_auth === "strong_auth";
  }
  return actionId === "prompt_control" || actionId === "main_token_transfer";
}
function buildSemanticCapability(actionId) {
  const observerOnly = selectedAgentInteractionMode() === "observer_only";
  const deploymentHint = authDeploymentHint(state.auth);
  const strongAuthSensitive = isStrongAuthSensitiveAction(actionId);
  const policy = hostedActionPolicy(actionId);
  if (observerOnly) {
    return {
      actionId,
      enabled: false,
      code: "observer_only",
      reason: "selected agent runs through OpenClaw(Local HTTP); software_safe stays observer-only for prompt/chat on this lane"
    };
  }
  if (policy) {
    if (policy.required_auth === "strong_auth") {
      const isLocalPreviewOnly = policy.availability === "trusted_local_preview_only";
      const isBackendGrantPreview = policy.availability === "public_player_plane_with_backend_reauth_preview";
      if (isLocalPreviewOnly && state.auth.available && !isHostedPublicJoinHint(deploymentHint)) {
        return {
          actionId,
          enabled: true,
          code: null,
          reason: policy.reason || "trusted local preview currently allows this strong-auth-marked action through preview bootstrap"
        };
      }
      if (isBackendGrantPreview && state.auth.available) {
        return {
          actionId,
          enabled: true,
          code: null,
          reason: policy.reason || `${actionId} is available through browser-local player auth plus backend re-authorization`
        };
      }
      if (isBackendGrantPreview && !state.auth.available) {
        return {
          actionId,
          enabled: false,
          code: "auth_level_insufficient",
          reason: `${actionId} requires player_session before backend re-authorization can upgrade it to strong_auth`
        };
      }
      return {
        actionId,
        enabled: false,
        code: "strong_auth_required",
        reason: policy.reason || strongAuthReason()
      };
    }
    if (!state.auth.available) {
      return {
        actionId,
        enabled: false,
        code: "auth_level_insufficient",
        reason: `${actionId} requires ${policy.required_auth}; current browser remains guest_session only`
      };
    }
    return {
      actionId,
      enabled: true,
      code: null,
      reason: policy.reason || `${actionId} is allowed on the ${policy.required_auth} lane`
    };
  }
  if (strongAuthSensitive && isHostedPublicJoinHint(deploymentHint)) {
    return {
      actionId,
      enabled: false,
      code: "strong_auth_required",
      reason: `${actionId} requires strong_auth on the hosted public join path; this browser is still guest_session only and the strong-auth upgrade lane is not implemented yet`
    };
  }
  if (strongAuthSensitive && state.auth.available && deploymentHint === "remote_origin_legacy_bootstrap") {
    return {
      actionId,
      enabled: false,
      code: "strong_auth_required",
      reason: `${actionId} is blocked on remote-origin legacy bootstrap; hosted/public prompt control must move to strong_auth or private operator plane`
    };
  }
  if (!state.auth.available) {
    const reason = isHostedPublicJoinHint(deploymentHint) ? `${actionId} requires player_session; this browser is still guest_session only on the hosted public join path` : `${actionId} requires viewer auth bootstrap; current status: ${state.auth.error || "missing"}`;
    return {
      actionId,
      enabled: false,
      code: "auth_level_insufficient",
      reason
    };
  }
  return {
    actionId,
    enabled: true,
    code: null,
    reason: strongAuthSensitive ? "prompt_control stays enabled only in trusted_local_preview via legacy viewer auth bootstrap; hosted/public strong_auth remains pending" : "player_session is active via legacy viewer auth bootstrap preview"
  };
}
function buildAuthSurfaceModel() {
  const deploymentHint = authDeploymentHint(state.auth);
  const promptCapability = buildSemanticCapability("prompt_control");
  const chatCapability = buildSemanticCapability("agent_chat");
  const mainTokenTransferCapability = buildSemanticCapability("main_token_transfer");
  const currentTier = state.auth.available ? "player_session" : "guest_session";
  const source = state.hostedAccess ? state.auth.available ? state.auth.source === "legacy_viewer_auth_bootstrap" ? "legacy_viewer_auth_bootstrap+hosted_access_hint" : "hosted_player_issue+browser_local_ephemeral_key" : "hosted_access_hint" : state.auth.available ? state.auth.source : "guest_only";
  return {
    deploymentHint,
    source,
    currentTier,
    currentTierReason: currentTier === "player_session" ? playerSessionReason(state.auth, deploymentHint) : guestSessionReason(state.auth, deploymentHint),
    tiers: [
      {
        id: "guest_session",
        label: "guest_session",
        status: state.auth.available ? "superseded" : "active",
        reason: guestSessionReason(state.auth, deploymentHint)
      },
      {
        id: "player_session",
        label: "player_session",
        status: state.auth.available ? state.auth.source === "legacy_viewer_auth_bootstrap" ? "active_legacy_preview" : state.auth.registrationStatus === "registered" ? "active_hosted_issue" : "issued_pending_register" : "not_issued",
        reason: playerSessionReason(state.auth, deploymentHint)
      },
      {
        id: "strong_auth",
        label: "strong_auth",
        status: "not_implemented",
        reason: strongAuthReason()
      }
    ],
    capabilities: {
      prompt_control: promptCapability,
      agent_chat: chatCapability,
      main_token_transfer: mainTokenTransferCapability,
      strong_auth_actions: mainTokenTransferCapability
    },
    reconnect: state.auth.available ? state.auth.source === "legacy_viewer_auth_bootstrap" ? "reconnect still depends on the current preview bootstrap; hosted resume/revoke tokens are not wired yet" : state.auth.registrationStatus === "registered" ? "page reload will reuse the browser-local hosted key and attempt reconnect_sync first" : "browser-local hosted key is persisted, but runtime session restore is still pending this page load" : "page reload is possible, but player-session reconnect/resume is not implemented yet"
  };
}
function buildHostedActionMatrixView() {
  const matrix = Array.isArray(state.hostedAccess?.action_matrix) ? state.hostedAccess.action_matrix : [];
  return matrix.map((policy) => {
    const actionId = String(policy?.action_id || "").trim();
    const capability = buildSemanticCapability(actionId);
    return {
      actionId,
      requiredAuth: String(policy?.required_auth || "").trim() || "unknown",
      availability: String(policy?.availability || "").trim() || "unknown",
      reason: String(policy?.reason || capability.reason || "").trim(),
      enabled: capability.enabled === true,
      code: capability.code || null,
      capabilityReason: capability.reason || null
    };
  });
}
function buildHostedRecoveryHint() {
  if (String(state.hostedAccess?.deployment_mode || "").trim() !== "hosted_public_join") {
    return null;
  }
  if (state.auth.available) {
    return null;
  }
  const errorText = String(state.auth.error || "").trim();
  const revokeReason = String(state.auth.revokeReason || "").trim();
  const revokedBy = String(state.auth.revokedBy || "").trim();
  if (!errorText) {
    return null;
  }
  if (errorText.includes("released locally")) {
    return {
      kind: "released",
      title: "Hosted player session released",
      detail: "This browser returned its hosted player slot locally. Acquire a new hosted player session if you want to resume gameplay.",
      cta: "Acquire Hosted Player Session"
    };
  }
  if (errorText.includes("revoked") || revokeReason || revokedBy) {
    const actorText = revokedBy ? ` by ${revokedBy}` : "";
    const reasonText = revokeReason ? ` Reason: ${revokeReason}.` : "";
    return {
      kind: "revoked",
      title: "Hosted player session was revoked",
      detail: `The runtime or operator revoked this browser session${actorText}.${reasonText} You need to acquire a fresh hosted player session before gameplay, chat, or prompt actions can continue.`,
      cta: "Re-acquire Hosted Player Session"
    };
  }
  if (errorText.includes("session_not_found") || errorText.includes("not found")) {
    return {
      kind: "missing",
      title: "Hosted player session is missing from runtime",
      detail: "The browser-local key still exists, but the runtime no longer recognizes the session. Acquire a fresh hosted player session and register again.",
      cta: "Re-acquire Hosted Player Session"
    };
  }
  return {
    kind: "guest",
    title: "Hosted player session is unavailable",
    detail: errorText,
    cta: "Acquire Hosted Player Session"
  };
}
function nextRequestId() {
  requestId += 1;
  return requestId;
}
function nextAuthNonce() {
  authNonceCounter += 1;
  return Date.now() + authNonceCounter;
}
function snapshotControlFeedback(feedback) {
  if (!feedback) return null;
  return {
    id: feedback.id,
    action: feedback.action,
    accepted: feedback.accepted,
    stage: feedback.stage,
    reason: feedback.reason,
    hint: feedback.hint,
    effect: feedback.effect,
    deltaLogicalTime: feedback.deltaLogicalTime || 0,
    deltaEventSeq: feedback.deltaEventSeq || 0,
    deltaTraceCount: feedback.deltaTraceCount || 0
  };
}
function snapshotSemanticFeedback(feedback) {
  if (!feedback) return null;
  return {
    id: feedback.id,
    kind: feedback.kind,
    action: feedback.action,
    agentId: feedback.agentId || null,
    accepted: feedback.accepted,
    stage: feedback.stage,
    ok: feedback.ok,
    reason: feedback.reason || null,
    effect: feedback.effect || null,
    response: clone(feedback.response) || null
  };
}
function getState() {
  const authSurface = buildAuthSurfaceModel();
  const hostedActionMatrixView = buildHostedActionMatrixView();
  const hostedRecoveryHint = buildHostedRecoveryHint();
  return {
    connectionStatus: state.connectionStatus,
    logicalTime: state.logicalTime,
    eventSeq: state.eventSeq,
    tick: state.tick,
    selectedKind: state.selectedKind,
    selectedId: state.selectedId,
    errorCount: state.errorCount,
    lastError: state.lastError,
    eventCount: state.eventCount,
    traceCount: state.traceCount,
    cameraMode: state.cameraMode,
    cameraRadius: state.cameraRadius,
    cameraOrthoScale: state.cameraOrthoScale,
    lastControlFeedback: snapshotControlFeedback(state.lastControlFeedback),
    lastPromptFeedback: snapshotSemanticFeedback(state.lastPromptFeedback),
    lastChatFeedback: snapshotSemanticFeedback(state.lastChatFeedback),
    renderMode: state.renderMode,
    rendererClass: state.rendererClass,
    softwareSafeReason: state.softwareSafeReason,
    renderer: state.renderer,
    vendor: state.vendor,
    webglVersion: state.webglVersion,
    controlProfile: state.controlProfile,
    debugViewerMode: state.debugViewerMode,
    debugViewerStatus: state.debugViewerStatus,
    worldId: state.worldId,
    server: state.server,
    wsUrl: state.wsUrl,
    authReady: state.auth.available,
    authPlayerId: state.auth.playerId,
    authPublicKey: state.auth.publicKey,
    authError: state.auth.error,
    authRevokeReason: state.auth.revokeReason,
    authRevokedBy: state.auth.revokedBy,
    authRegistrationStatus: state.auth.registrationStatus,
    authSessionEpoch: state.auth.sessionEpoch,
    authRecoveryErrorCode: state.auth.recoveryErrorCode,
    authRecoveryErrorMessage: state.auth.recoveryErrorMessage,
    authRuntimeStatus: state.auth.runtimeStatus,
    authBoundAgentId: state.auth.boundAgentId,
    authPendingRequestedAgentId: state.auth.pendingRequestedAgentId,
    authPendingForceRebind: state.auth.pendingForceRebind,
    authRebindNotice: state.auth.rebindNotice,
    authTier: authSurface.currentTier,
    authSource: authSurface.source,
    authDeploymentHint: authSurface.deploymentHint,
    authSurface: clone(authSurface),
    hostedRecoveryHint: clone(hostedRecoveryHint),
    hostedAccess: clone(state.hostedAccess),
    hostedActionMatrix: clone(hostedActionMatrixView),
    hostedAdmission: clone(state.hostedAdmission),
    strongAuthApprovalCodeConfigured: !!String(state.strongAuth.approvalCode || "").trim(),
    strongAuthLastGrantActionId: state.strongAuth.lastGrantActionId,
    strongAuthLastGrantExpiresAtUnixMs: state.strongAuth.lastGrantExpiresAtUnixMs,
    strongAuthLastGrantError: state.strongAuth.lastGrantError,
    selectedAgentInteractionMode: selectedAgentInteractionMode(),
    selectedAgentDebug: clone(selectedAgentExecutionDebugContext()),
    selectedPromptVersion: state.promptDraft.currentVersion || 0,
    promptRollbackTargetVersion: state.promptDraft.rollbackTargetVersion || 0,
    chatHistoryCount: state.chatHistory.length,
    chatHistory: clone(state.chatHistory)
  };
}
function reportFatalError(message, source = "runtime") {
  const text = `${source}: ${String(message || "unknown runtime error")}`.trim();
  if (state.lastError !== text) {
    state.errorCount += 1;
  }
  state.connectionStatus = "error";
  state.debugViewerStatus = "error";
  state.lastError = text;
  render();
}
function parseSelectionPayload(payload) {
  if (payload == null) {
    return null;
  }
  if (typeof payload === "string") {
    const trimmed = payload.trim();
    if (!trimmed) return null;
    const parts = trimmed.split(":");
    if (parts.length >= 2) {
      return { kind: parts[0], id: parts.slice(1).join(":") };
    }
    return { kind: "agent", id: trimmed };
  }
  if (typeof payload === "object") {
    const kind = payload.kind || payload.targetKind || payload.type;
    const id = payload.id || payload.targetId || payload.value;
    if (!kind || !id) return null;
    return { kind: String(kind), id: String(id) };
  }
  return null;
}
function entityCollections() {
  const model = state.snapshot?.model || {};
  return {
    agents: Object.values(model.agents || {}),
    locations: Object.values(model.locations || {})
  };
}
function selectedAgentId() {
  return state.selectedKind === "agent" ? state.selectedId : null;
}
function selectedAgentPromptProfile() {
  const agentId = selectedAgentId();
  if (!agentId) {
    return null;
  }
  return state.snapshot?.model?.agent_prompt_profiles?.[agentId] || {
    agent_id: agentId,
    version: 0,
    updated_at_tick: 0,
    updated_by: "",
    system_prompt_override: null,
    short_term_goal_override: null,
    long_term_goal_override: null
  };
}
function selectedAgentBindingInfo() {
  const agentId = selectedAgentId();
  if (!agentId) {
    return null;
  }
  return {
    playerId: state.snapshot?.model?.agent_player_bindings?.[agentId] || null,
    publicKey: state.snapshot?.model?.agent_player_public_key_bindings?.[agentId] || null
  };
}
function selectedAgentExecutionDebugContext() {
  const agentId = selectedAgentId();
  if (!agentId) {
    return null;
  }
  return state.snapshot?.model?.agent_execution_debug_contexts?.[agentId] || null;
}
function selectedAgentInteractionMode() {
  const debugContext = selectedAgentExecutionDebugContext();
  if (debugContext?.provider_mode === "openclaw_local_http") {
    return "observer_only";
  }
  return "interactive";
}
function syncAgentInteractionDrafts(force = false) {
  const agentId = selectedAgentId();
  const profile = selectedAgentPromptProfile();
  if (force || state.promptDraft.agentId !== agentId || !state.promptDraft.dirty && agentId) {
    const currentVersion = Number(profile?.version || 0);
    state.promptDraft = {
      agentId,
      currentVersion,
      rollbackTargetVersion: Math.max(0, currentVersion - 1),
      updatedBy: String(profile?.updated_by || ""),
      updatedAtTick: Number(profile?.updated_at_tick || 0),
      systemPrompt: String(profile?.system_prompt_override || ""),
      shortTermGoal: String(profile?.short_term_goal_override || ""),
      longTermGoal: String(profile?.long_term_goal_override || ""),
      dirty: false
    };
  }
  if (force || state.chatDraft.agentId !== agentId) {
    state.chatDraft = {
      agentId,
      message: agentId === state.chatDraft.agentId ? state.chatDraft.message : "",
      dirty: false
    };
  }
}
function applySelection(selection) {
  if (!selection) return null;
  const kind = String(selection.kind || "").toLowerCase();
  const id = String(selection.id || "");
  const { agents, locations } = entityCollections();
  let object = null;
  if (kind === "agent") {
    object = agents.find((entry) => entry.id === id) || null;
  } else if (kind === "location") {
    object = locations.find((entry) => entry.id === id) || null;
  }
  if (!object) {
    return null;
  }
  state.selectedKind = kind;
  state.selectedId = id;
  state.selectedObject = object;
  syncAgentInteractionDrafts(true);
  render();
  return { kind, id };
}
function select(payload) {
  const parsed = parseSelectionPayload(payload);
  if (!parsed) {
    return { ok: false, reason: "invalid selection payload" };
  }
  const applied = applySelection(parsed);
  if (!applied) {
    return { ok: false, reason: `target not found: ${parsed.kind}:${parsed.id}` };
  }
  return { ok: true, ...applied };
}
function focus(payload) {
  return select(payload);
}
function parseStepCount(payload) {
  if (payload == null) return 1;
  if (typeof payload === "number" && Number.isFinite(payload) && payload >= 1) {
    return Math.floor(payload);
  }
  if (typeof payload === "string") {
    const trimmed = payload.trim();
    if (!trimmed || trimmed === "step") return 1;
    const numeric = Number(trimmed);
    if (Number.isFinite(numeric) && numeric >= 1) {
      return Math.floor(numeric);
    }
    const matched = trimmed.match(/step\s*[:=]\s*(\d+)/i);
    if (matched) {
      return Number(matched[1]);
    }
    return null;
  }
  if (typeof payload === "object") {
    const numeric = Number(payload.count);
    if (Number.isFinite(numeric) && numeric >= 1) {
      return Math.floor(numeric);
    }
  }
  return null;
}
function controlActions() {
  return [
    {
      action: "play",
      description: "Start continuous world advancement",
      descriptionZh: "开始连续推进世界",
      examplePayload: null
    },
    {
      action: "pause",
      description: "Pause continuous advancement",
      descriptionZh: "暂停连续推进",
      examplePayload: null
    },
    {
      action: "step",
      description: "Advance fixed steps (payload.count)",
      descriptionZh: "推进固定步数（payload.count）",
      examplePayload: { count: 5 }
    }
  ];
}
function describeControls() {
  return {
    controls: controlActions(),
    semanticActions: [
      {
        action: "sendAgentChat",
        description: "Send a player-authenticated chat message to an agent"
      },
      {
        action: "sendPromptControl",
        description: "Preview, apply, or rollback prompt overrides for an agent"
      }
    ],
    usage: "Use fillControlExample(action), sendControl(action), sendAgentChat(agentId, message), sendPromptControl(mode, payload).",
    notes: [
      "software_safe acts as a debug_viewer lane: it subscribes to runtime snapshots/events and does not own world authority",
      "when selectedAgentDebug.provider_mode=openclaw_local_http, prompt/chat stay observer-only in runtime live",
      "without viewer auth bootstrap the browser stays guest_session only; hosted public join player-session issuance is still pending"
    ]
  };
}
function fillControlExample(action) {
  const normalized = String(action || "").trim().toLowerCase();
  return controlActions().find((entry) => entry.action === normalized)?.examplePayload ?? null;
}
function sendJson(payload) {
  if (!socket || socket.readyState !== WebSocket.OPEN) {
    throw new Error("viewer websocket is not connected");
  }
  socket.send(JSON.stringify(payload));
}
function sendViewerControl(action, payload) {
  const normalized = String(action || "").trim().toLowerCase();
  const currentRequestId = nextRequestId();
  const feedback = {
    id: currentRequestId,
    action: normalized,
    accepted: false,
    stage: "rejected",
    reason: null,
    hint: null,
    effect: null,
    baselineLogicalTime: state.logicalTime,
    baselineEventSeq: state.eventSeq,
    deltaLogicalTime: 0,
    deltaEventSeq: 0,
    deltaTraceCount: 0,
    requestId: currentRequestId
  };
  let mode = null;
  if (normalized === "play") {
    mode = { mode: "play" };
  } else if (normalized === "pause") {
    mode = { mode: "pause" };
  } else if (normalized === "step") {
    const count = parseStepCount(payload);
    if (!count) {
      feedback.reason = "step requires numeric payload.count >= 1";
      feedback.effect = "request rejected before send";
      state.lastControlFeedback = feedback;
      render();
      return snapshotControlFeedback(feedback);
    }
    mode = { mode: "step", count };
  } else {
    feedback.reason = `unsupported action: ${normalized}`;
    feedback.effect = "request rejected before send";
    state.lastControlFeedback = feedback;
    render();
    return snapshotControlFeedback(feedback);
  }
  try {
    if (state.controlProfile === "live") {
      sendJson({ type: "live_control", mode, request_id: currentRequestId });
    } else if (state.controlProfile === "playback") {
      sendJson({ type: "playback_control", mode, request_id: currentRequestId });
    } else {
      sendJson({ type: "control", mode, request_id: currentRequestId });
    }
    feedback.accepted = true;
    feedback.stage = "queued";
    feedback.effect = "queued, check getState().lastControlFeedback for world delta";
    pendingControlFeedback.set(currentRequestId, feedback);
    state.lastControlFeedback = feedback;
    render();
    return snapshotControlFeedback(feedback);
  } catch (error) {
    feedback.reason = String(error);
    feedback.effect = "request send failed";
    state.lastControlFeedback = feedback;
    render();
    return snapshotControlFeedback(feedback);
  }
}
function sendControl(action, payload = null) {
  return sendViewerControl(action, payload);
}
function runSteps(payload) {
  const count = parseStepCount(payload);
  if (!count) {
    return { ok: false, reason: "payload must be non-empty step string or count" };
  }
  const feedback = sendControl("step", { count });
  return { ok: Boolean(feedback?.accepted), count, feedback };
}
function setMode() {
  return {
    ok: false,
    reason: "software_safe viewer does not expose 2d/3d camera modes"
  };
}
function updateControlFeedbackFromProgress() {
  const feedback = state.lastControlFeedback;
  if (!feedback || !feedback.accepted) return;
  const deltaLogicalTime = Math.max(0, state.logicalTime - feedback.baselineLogicalTime);
  const deltaEventSeq = Math.max(0, state.eventSeq - feedback.baselineEventSeq);
  feedback.deltaLogicalTime = deltaLogicalTime;
  feedback.deltaEventSeq = deltaEventSeq;
  if (deltaLogicalTime > 0 || deltaEventSeq > 0) {
    feedback.stage = "completed_advanced";
    feedback.effect = `world advanced: logicalTime +${deltaLogicalTime}, eventSeq +${deltaEventSeq}`;
  }
}
function summarizeEventTitle(event) {
  const kind = event?.kind?.type || "unknown";
  return kind.replace(/_/g, " ");
}
function addRecentEvent(event) {
  state.recentEvents.unshift(event);
  state.recentEvents = state.recentEvents.slice(0, MAX_EVENTS);
  state.eventCount = state.recentEvents.length;
  state.eventSeq = Math.max(state.eventSeq, Number(event?.id || 0));
}
function handleSnapshot(snapshot) {
  state.snapshot = snapshot;
  state.logicalTime = Math.max(state.logicalTime, Number(snapshot?.time || 0));
  state.tick = state.logicalTime;
  const { agents, locations } = entityCollections();
  if (!state.selectedObject) {
    if (agents[0]) {
      applySelection({ kind: "agent", id: agents[0].id });
    } else if (locations[0]) {
      applySelection({ kind: "location", id: locations[0].id });
    }
  } else if (state.selectedKind && state.selectedId) {
    applySelection({ kind: state.selectedKind, id: state.selectedId });
  }
  syncAgentInteractionDrafts(false);
}
function handleMetrics(time, metrics) {
  state.metrics = metrics || null;
  state.traceCount = Number(metrics?.decision_trace_count || 0);
  state.logicalTime = Math.max(state.logicalTime, Number(time || 0), Number(metrics?.total_ticks || 0));
  state.tick = state.logicalTime;
}
function handleControlCompletionAck(ack) {
  const feedback = pendingControlFeedback.get(ack?.request_id) || state.lastControlFeedback;
  if (!feedback) return;
  feedback.deltaLogicalTime = Number(ack?.delta_logical_time || 0);
  feedback.deltaEventSeq = Number(ack?.delta_event_seq || 0);
  feedback.stage = ack?.status === "advanced" ? "completed_advanced" : "completed_timeout";
  feedback.effect = ack?.status === "advanced" ? `control ack advanced: logicalTime +${feedback.deltaLogicalTime}, eventSeq +${feedback.deltaEventSeq}` : "control ack timed out without progress";
  if (ack?.status !== "advanced") {
    feedback.reason = "timeout_no_progress";
  }
  state.lastControlFeedback = feedback;
  pendingControlFeedback.delete(feedback.requestId);
}
function cborHeader(majorType, length) {
  if (!Number.isInteger(length) || length < 0) {
    throw new Error(`invalid CBOR length: ${length}`);
  }
  if (length < 24) {
    return Uint8Array.of(majorType << 5 | length);
  }
  if (length < 256) {
    return Uint8Array.of(majorType << 5 | 24, length);
  }
  if (length < 65536) {
    return Uint8Array.of(majorType << 5 | 25, length >> 8 & 255, length & 255);
  }
  if (length <= 4294967295) {
    return Uint8Array.of(
      majorType << 5 | 26,
      length >>> 24 & 255,
      length >>> 16 & 255,
      length >>> 8 & 255,
      length & 255
    );
  }
  if (length <= Number.MAX_SAFE_INTEGER) {
    const value = BigInt(length);
    return Uint8Array.of(
      majorType << 5 | 27,
      Number(value >> 56n & 0xffn),
      Number(value >> 48n & 0xffn),
      Number(value >> 40n & 0xffn),
      Number(value >> 32n & 0xffn),
      Number(value >> 24n & 0xffn),
      Number(value >> 16n & 0xffn),
      Number(value >> 8n & 0xffn),
      Number(value & 0xffn)
    );
  }
  throw new Error("CBOR length exceeds Number.MAX_SAFE_INTEGER");
}
function concatBytes(...parts) {
  const totalLength = parts.reduce((sum, bytes) => sum + bytes.length, 0);
  const out = new Uint8Array(totalLength);
  let offset = 0;
  for (const bytes of parts) {
    out.set(bytes, offset);
    offset += bytes.length;
  }
  return out;
}
function cborEncode(value) {
  if (value === null) {
    return Uint8Array.of(246);
  }
  if (value === false) {
    return Uint8Array.of(244);
  }
  if (value === true) {
    return Uint8Array.of(245);
  }
  if (typeof value === "number") {
    if (!Number.isInteger(value) || value < 0) {
      throw new Error(`unsupported CBOR number: ${value}`);
    }
    return cborHeader(0, value);
  }
  if (typeof value === "string") {
    const bytes = textEncoder.encode(value);
    return concatBytes(cborHeader(3, bytes.length), bytes);
  }
  if (Array.isArray(value)) {
    return concatBytes(cborHeader(4, value.length), ...value.map((entry) => cborEncode(entry)));
  }
  if (value instanceof Uint8Array) {
    return concatBytes(cborHeader(2, value.length), value);
  }
  if (typeof value === "object") {
    const entries = Object.entries(value).filter(([, entryValue]) => entryValue !== void 0);
    const encoded = [cborHeader(5, entries.length)];
    for (const [key, entryValue] of entries) {
      encoded.push(cborEncode(String(key)));
      encoded.push(cborEncode(entryValue));
    }
    return concatBytes(...encoded);
  }
  throw new Error(`unsupported CBOR type: ${typeof value}`);
}
function hexToBytes(raw) {
  const value = String(raw || "").trim().toLowerCase();
  if (!value || value.length % 2 !== 0 || /[^0-9a-f]/.test(value)) {
    throw new Error("invalid hex payload");
  }
  const bytes = new Uint8Array(value.length / 2);
  for (let index = 0; index < bytes.length; index += 1) {
    bytes[index] = Number.parseInt(value.slice(index * 2, index * 2 + 2), 16);
  }
  return bytes;
}
function bytesToHex(bytes) {
  return Array.from(bytes, (value) => value.toString(16).padStart(2, "0")).join("");
}
function bytesStartWith(bytes, prefix) {
  if (bytes.length < prefix.length) {
    return false;
  }
  for (let index = 0; index < prefix.length; index += 1) {
    if (bytes[index] !== prefix[index]) {
      return false;
    }
  }
  return true;
}
async function importEd25519SigningKey(privateKeyHex) {
  if (!window.crypto?.subtle) {
    throw new Error("Web Crypto subtle API is unavailable");
  }
  if (!authKeyCache.has(privateKeyHex)) {
    const rawPrivateKey = hexToBytes(privateKeyHex);
    if (rawPrivateKey.length !== 32) {
      throw new Error(`viewer auth private key length mismatch: expected 32 bytes, got ${rawPrivateKey.length}`);
    }
    const pkcs8 = concatBytes(ED25519_PKCS8_PREFIX, rawPrivateKey);
    authKeyCache.set(
      privateKeyHex,
      window.crypto.subtle.importKey("pkcs8", pkcs8, { name: "Ed25519" }, false, ["sign"])
    );
  }
  return authKeyCache.get(privateKeyHex);
}
async function signAuthPayload(signingPayloadBytes, auth) {
  const key = await importEd25519SigningKey(auth.privateKey);
  const signature = await window.crypto.subtle.sign({ name: "Ed25519" }, key, signingPayloadBytes);
  return `${VIEWER_AUTH_SIGNATURE_PREFIX}${bytesToHex(new Uint8Array(signature))}`;
}
async function generateEphemeralEd25519Keypair() {
  if (!window.crypto?.subtle) {
    throw new Error("Web Crypto subtle API is unavailable");
  }
  const keyPair = await window.crypto.subtle.generateKey(
    { name: "Ed25519" },
    true,
    ["sign", "verify"]
  );
  const pkcs8 = new Uint8Array(await window.crypto.subtle.exportKey("pkcs8", keyPair.privateKey));
  if (!bytesStartWith(pkcs8, ED25519_PKCS8_PREFIX) || pkcs8.length !== ED25519_PKCS8_PREFIX.length + 32) {
    throw new Error("unexpected Ed25519 pkcs8 encoding from Web Crypto");
  }
  const rawPublicKey = new Uint8Array(await window.crypto.subtle.exportKey("raw", keyPair.publicKey));
  if (rawPublicKey.length !== 32) {
    throw new Error(`unexpected Ed25519 public key length: ${rawPublicKey.length}`);
  }
  return {
    publicKey: bytesToHex(rawPublicKey),
    privateKey: bytesToHex(pkcs8.slice(ED25519_PKCS8_PREFIX.length))
  };
}
function buildAuthEnvelope(payload) {
  return cborEncode({
    version: 1,
    payload
  });
}
async function buildAgentChatAuthProof(request, auth) {
  const nonce = nextAuthNonce();
  const payload = {
    operation: "agent_chat",
    agent_id: request.agent_id,
    player_id: auth.playerId,
    public_key: auth.publicKey,
    nonce,
    message: request.message
  };
  if (request.intent_tick != null) {
    payload.intent_tick = request.intent_tick;
  }
  if (request.intent_seq != null) {
    payload.intent_seq = request.intent_seq;
  }
  const signingPayload = buildAuthEnvelope(payload);
  return {
    scheme: "ed25519",
    player_id: auth.playerId,
    public_key: auth.publicKey,
    nonce,
    signature: await signAuthPayload(signingPayload, auth)
  };
}
function promptPatchFromDraft(currentValue, draftValue) {
  const current = currentValue == null ? "" : String(currentValue);
  const draft = String(draftValue ?? "");
  if (draft === current) {
    return { mode: "unchanged" };
  }
  if (draft.length === 0) {
    return currentValue == null ? { mode: "unchanged" } : { mode: "clear" };
  }
  return { mode: "set", value: draft };
}
async function buildPromptControlAuthProof(mode, request, auth) {
  const nonce = nextAuthNonce();
  const payload = {
    operation: mode === "preview" ? "prompt_control_preview" : "prompt_control_apply",
    agent_id: request.agent_id,
    player_id: auth.playerId,
    public_key: auth.publicKey,
    nonce,
    expected_version: request.expected_version ?? null,
    updated_by: request.updated_by ?? null,
    system_prompt_override: request.system_prompt_override,
    short_term_goal_override: request.short_term_goal_override,
    long_term_goal_override: request.long_term_goal_override
  };
  const signingPayload = buildAuthEnvelope(payload);
  return {
    scheme: "ed25519",
    player_id: auth.playerId,
    public_key: auth.publicKey,
    nonce,
    signature: await signAuthPayload(signingPayload, auth)
  };
}
async function buildPromptRollbackAuthProof(request, auth) {
  const nonce = nextAuthNonce();
  const payload = {
    operation: "prompt_control_rollback",
    agent_id: request.agent_id,
    player_id: auth.playerId,
    public_key: auth.publicKey,
    nonce,
    to_version: request.to_version,
    expected_version: request.expected_version ?? null,
    updated_by: request.updated_by ?? null
  };
  const signingPayload = buildAuthEnvelope(payload);
  return {
    scheme: "ed25519",
    player_id: auth.playerId,
    public_key: auth.publicKey,
    nonce,
    signature: await signAuthPayload(signingPayload, auth)
  };
}
async function buildSessionRegisterAuthProof(request, auth) {
  const nonce = nextAuthNonce();
  const payload = {
    operation: "session_register",
    player_id: auth.playerId,
    public_key: auth.publicKey,
    nonce
  };
  if (request.requested_agent_id != null) {
    payload.requested_agent_id = request.requested_agent_id;
  }
  payload.force_rebind = request.force_rebind === true;
  const signingPayload = buildAuthEnvelope(payload);
  return {
    scheme: "ed25519",
    player_id: auth.playerId,
    public_key: auth.publicKey,
    nonce,
    signature: await signAuthPayload(signingPayload, auth)
  };
}
function canAutoIssueHostedPlayerSession() {
  return String(state.hostedAccess?.deployment_mode || "").trim() === "hosted_public_join" && state.auth.source !== "legacy_viewer_auth_bootstrap";
}
async function issueHostedPlayerIdentity() {
  if (!canAutoIssueHostedPlayerSession()) {
    return state.auth;
  }
  if (state.auth.available || state.auth.issueInFlight) {
    return state.auth;
  }
  state.auth.issueInFlight = true;
  state.auth.error = null;
  render();
  try {
    const response = await fetch(HOSTED_PLAYER_SESSION_ISSUE_ROUTE, {
      method: "GET",
      cache: "no-store",
      headers: { Accept: "application/json" }
    });
    const payload = await response.json();
    if (!response.ok || !payload?.ok || !payload?.grant?.player_id) {
      if (payload?.admission) {
        state.hostedAdmission = clone(payload.admission);
      }
      throw new Error(payload?.error || payload?.error_code || `hosted player-session issue failed with HTTP ${response.status}`);
    }
    state.hostedAdmission = payload?.admission ? clone(payload.admission) : state.hostedAdmission;
    const keypair = await generateEphemeralEd25519Keypair();
    state.auth = {
      available: true,
      playerId: String(payload.grant.player_id || "").trim(),
      publicKey: keypair.publicKey,
      privateKey: keypair.privateKey,
      releaseToken: String(payload.grant.release_token || "").trim() || null,
      error: null,
      revokeReason: null,
      revokedBy: null,
      source: "hosted_browser_storage",
      registrationStatus: "issued",
      sessionEpoch: null,
      issuedAtUnixMs: payload?.grant?.issued_at_unix_ms == null ? Date.now() : Number(payload.grant.issued_at_unix_ms),
      recoveryErrorCode: null,
      recoveryErrorMessage: null,
      issueInFlight: false,
      syncInFlight: false,
      runtimeStatus: "issued",
      boundAgentId: null,
      pendingRequestedAgentId: null,
      pendingForceRebind: false,
      rebindNotice: null
    };
    persistHostedPlayerSession(state.auth);
    render();
    return state.auth;
  } catch (error) {
    state.auth.issueInFlight = false;
    state.auth.error = String(error);
    render();
    return state.auth;
  }
}
async function ensureHostedPlayerAuthAvailable() {
  if (state.auth.available) {
    return state.auth;
  }
  if (canAutoIssueHostedPlayerSession()) {
    return issueHostedPlayerIdentity();
  }
  return state.auth;
}
async function retryHostedPlayerIdentityIssue() {
  if (!canAutoIssueHostedPlayerSession()) {
    return { ok: false, reason: "hosted public player-session issue is unavailable on this lane" };
  }
  const auth = await issueHostedPlayerIdentity();
  render();
  return {
    ok: auth.available,
    playerId: auth.playerId,
    error: auth.error
  };
}
async function requestHostedStrongAuthGrant(actionId, agentId) {
  const playerId = String(state.auth.playerId || "").trim();
  const publicKey = String(state.auth.publicKey || "").trim();
  const releaseToken = String(state.auth.releaseToken || "").trim();
  const approvalCode = String(state.strongAuth.approvalCode || "").trim();
  if (!playerId || !publicKey || !releaseToken) {
    throw new Error("hosted strong-auth grant requires an active player_session with release token");
  }
  if (!approvalCode) {
    throw new Error("backend approval code is required before hosted strong auth can be granted");
  }
  const query = new URLSearchParams({
    player_id: playerId,
    public_key: publicKey,
    release_token: releaseToken,
    agent_id: String(agentId || "").trim(),
    action_id: String(actionId || "").trim(),
    approval_code: approvalCode
  });
  const response = await fetch(`${HOSTED_STRONG_AUTH_GRANT_ROUTE}?${query.toString()}`, {
    method: "GET",
    cache: "no-store",
    headers: { Accept: "application/json" }
  });
  const payload = await response.json();
  if (payload?.admission) {
    state.hostedAdmission = clone(payload.admission);
  }
  if (!response.ok || !payload?.ok || !payload?.grant) {
    state.strongAuth.lastGrantError = payload?.error || payload?.error_code || `hosted strong-auth grant failed with HTTP ${response.status}`;
    throw new Error(state.strongAuth.lastGrantError);
  }
  state.strongAuth.lastGrantActionId = String(payload.grant.action_id || "").trim() || actionId;
  state.strongAuth.lastGrantExpiresAtUnixMs = payload?.grant?.expires_at_unix_ms == null ? null : Number(payload.grant.expires_at_unix_ms);
  state.strongAuth.lastGrantError = null;
  return payload.grant;
}
function sendReconnectSync() {
  if (!state.auth.available || state.auth.source === "legacy_viewer_auth_bootstrap") {
    return;
  }
  state.auth.syncInFlight = true;
  state.auth.registrationStatus = "registering";
  state.auth.runtimeStatus = "probing";
  state.auth.recoveryErrorCode = null;
  state.auth.recoveryErrorMessage = null;
  sendJson({
    type: "authoritative_recovery",
    command: {
      mode: "reconnect_sync",
      request: {
        player_id: state.auth.playerId,
        session_pubkey: state.auth.publicKey
      }
    }
  });
}
function probeHostedRuntimeSession() {
  if (!state.auth.available || state.auth.source === "legacy_viewer_auth_bootstrap" || state.connectionStatus !== "connected" || state.auth.registrationStatus !== "registered") {
    return;
  }
  state.auth.syncInFlight = true;
  state.auth.runtimeStatus = "probing";
  sendJson({
    type: "authoritative_recovery",
    command: {
      mode: "reconnect_sync",
      request: {
        player_id: state.auth.playerId,
        session_pubkey: state.auth.publicKey
      }
    }
  });
}
async function releaseHostedPlayerSlot() {
  const playerId = String(state.auth.playerId || "").trim();
  const releaseToken = String(state.auth.releaseToken || "").trim();
  if (!playerId || !releaseToken || state.auth.source === "legacy_viewer_auth_bootstrap") {
    return { ok: false, skipped: true };
  }
  const query = `player_id=${encodeURIComponent(playerId)}&release_token=${encodeURIComponent(releaseToken)}`;
  const response = await fetch(`${HOSTED_PLAYER_SESSION_RELEASE_ROUTE}?${query}`, {
    method: "POST",
    cache: "no-store",
    headers: { Accept: "application/json" }
  });
  const payload = await response.json();
  if (!response.ok || !payload?.ok) {
    if (payload?.admission) {
      state.hostedAdmission = clone(payload.admission);
    }
    throw new Error(payload?.error || payload?.error_code || `hosted player-session release failed with HTTP ${response.status}`);
  }
  state.hostedAdmission = payload?.admission ? clone(payload.admission) : state.hostedAdmission;
  return payload;
}
function resetHostedPlayerAuthState(errorMessage = null, revocationMeta = null) {
  stopHostedSessionRefreshLoop();
  clearHostedPlayerSession();
  const bootstrap2 = resolveAuthBootstrap();
  const revokeReason = String(revocationMeta?.revokeReason || "").trim() || null;
  const revokedBy = String(revocationMeta?.revokedBy || "").trim() || null;
  state.auth = bootstrap2.available ? bootstrap2 : {
    ...bootstrap2,
    source: "guest_only",
    registrationStatus: "guest",
    error: errorMessage,
    revokeReason,
    revokedBy,
    sessionEpoch: null,
    issuedAtUnixMs: null,
    releaseToken: null,
    recoveryErrorCode: null,
    recoveryErrorMessage: null,
    issueInFlight: false,
    syncInFlight: false,
    runtimeStatus: "guest",
    boundAgentId: null,
    pendingRequestedAgentId: null,
    pendingForceRebind: false,
    rebindNotice: null
  };
  void refreshHostedAdmissionState().then(() => render());
}
async function logoutHostedPlayerSession() {
  if (!state.auth.available || state.auth.source === "legacy_viewer_auth_bootstrap") {
    return { ok: false, reason: "hosted browser session is unavailable" };
  }
  const revokeRequest = {
    player_id: state.auth.playerId,
    session_pubkey: state.auth.publicKey,
    revoke_reason: "player_logout",
    revoked_by: state.auth.playerId
  };
  try {
    if (state.connectionStatus === "connected") {
      sendJson({
        type: "authoritative_recovery",
        command: {
          mode: "revoke_session",
          request: revokeRequest
        }
      });
    }
  } catch (_) {
  }
  try {
    await releaseHostedPlayerSlot();
  } finally {
    resetHostedPlayerAuthState("hosted player session released locally");
    render();
  }
  return { ok: true };
}
function syncHostedPlayerSessionOnConnect() {
  if (!state.auth.available || state.auth.source === "legacy_viewer_auth_bootstrap" || state.auth.syncInFlight) {
    return;
  }
  sendReconnectSync();
}
function clearPendingSessionRegisterWaiter(error = null) {
  if (!pendingSessionRegisterWaiter) {
    return;
  }
  const waiter = pendingSessionRegisterWaiter;
  pendingSessionRegisterWaiter = null;
  if (error != null) {
    waiter.reject(error instanceof Error ? error : new Error(String(error)));
  }
}
async function dispatchSessionRegisterRequest(requestedAgentId, forceRebind) {
  const normalizedRequestedAgentId = String(requestedAgentId || "").trim() || null;
  if (state.auth.source !== "legacy_viewer_auth_bootstrap") {
    state.auth.registrationStatus = "registering";
    state.auth.syncInFlight = true;
    state.auth.recoveryErrorCode = null;
    state.auth.recoveryErrorMessage = null;
    state.auth.runtimeStatus = forceRebind === true ? "rebind_registering" : "registering";
  }
  if (forceRebind === true) {
    state.auth.rebindNotice = `Switching player session to ${normalizedRequestedAgentId || "requested agent"}...`;
  }
  state.auth.pendingRequestedAgentId = normalizedRequestedAgentId;
  state.auth.pendingForceRebind = forceRebind === true;
  const request = {
    player_id: state.auth.playerId,
    public_key: state.auth.publicKey
  };
  if (normalizedRequestedAgentId) {
    request.requested_agent_id = normalizedRequestedAgentId;
  }
  if (forceRebind === true) {
    request.force_rebind = true;
  }
  request.auth = await buildSessionRegisterAuthProof(request, state.auth);
  sendJson({
    type: "authoritative_recovery",
    command: {
      mode: "register_session",
      request
    }
  });
  render();
}
async function retryPendingSessionRegisterWaiterWithForceRebind() {
  const waiter = pendingSessionRegisterWaiter;
  if (!waiter) {
    return;
  }
  waiter.forceRebind = true;
  try {
    await dispatchSessionRegisterRequest(waiter.requestedAgentId, true);
  } catch (error) {
    clearPendingSessionRegisterWaiter(error);
    throw error;
  }
}
function latestRequestedAgentId(fallbackAgentId = null) {
  const agentId = String(
    fallbackAgentId || state.auth.pendingRequestedAgentId || state.auth.boundAgentId || ""
  ).trim();
  return agentId || null;
}
function recoveryErrorRequiresExplicitRebind(error) {
  return String(error?.code || "").trim() === "player_bind_failed" && String(error?.message || "").includes("explicit rebind required");
}
async function ensureRegisteredPlayerSession(requestedAgentId = null, options = {}) {
  await ensureHostedPlayerAuthAvailable();
  if (!state.auth.available) {
    throw new Error(state.auth.error || "player session auth is unavailable");
  }
  const normalizedRequestedAgentId = String(requestedAgentId || "").trim() || null;
  const forceRebind = options?.forceRebind === true;
  if (state.auth.registrationStatus === "registered" && (state.auth.runtimeStatus === "registered" || state.auth.runtimeStatus === "registered_unbound") && !forceRebind && (normalizedRequestedAgentId == null || normalizedRequestedAgentId === state.auth.boundAgentId)) {
    return state.auth;
  }
  if (pendingSessionRegisterWaiter) {
    const sameRequest = pendingSessionRegisterWaiter.requestedAgentId === normalizedRequestedAgentId && pendingSessionRegisterWaiter.forceRebind === forceRebind;
    if (!sameRequest) {
      throw new Error("another player session registration is already in flight");
    }
    return pendingSessionRegisterWaiter.promise;
  }
  let resolveWaiter;
  let rejectWaiter;
  const promise = new Promise((resolve, reject) => {
    resolveWaiter = resolve;
    rejectWaiter = reject;
  });
  pendingSessionRegisterWaiter = {
    requestedAgentId: normalizedRequestedAgentId,
    forceRebind,
    promise,
    resolve: resolveWaiter,
    reject: rejectWaiter
  };
  try {
    await dispatchSessionRegisterRequest(normalizedRequestedAgentId, forceRebind);
  } catch (error) {
    clearPendingSessionRegisterWaiter(error);
    throw error;
  }
  return promise;
}
function buildPromptRequestFromDraft(agentId, draftOverrides) {
  const currentProfile = selectedAgentPromptProfile();
  if (!agentId || !currentProfile) {
    throw new Error("select an agent before editing prompt overrides");
  }
  return {
    agent_id: agentId,
    player_id: state.auth.playerId,
    public_key: state.auth.publicKey,
    expected_version: Number(currentProfile.version || 0),
    updated_by: state.auth.playerId,
    system_prompt_override: promptPatchFromDraft(currentProfile.system_prompt_override, draftOverrides.systemPrompt),
    short_term_goal_override: promptPatchFromDraft(currentProfile.short_term_goal_override, draftOverrides.shortTermGoal),
    long_term_goal_override: promptPatchFromDraft(currentProfile.long_term_goal_override, draftOverrides.longTermGoal)
  };
}
function encodePromptRequestForJson(request) {
  const encodePatch = (patch) => {
    if (!patch || patch.mode === "unchanged") {
      return void 0;
    }
    if (patch.mode === "clear") {
      return null;
    }
    return patch.value;
  };
  return {
    agent_id: request.agent_id,
    player_id: request.player_id,
    public_key: request.public_key,
    expected_version: request.expected_version,
    updated_by: request.updated_by,
    system_prompt_override: encodePatch(request.system_prompt_override),
    short_term_goal_override: encodePatch(request.short_term_goal_override),
    long_term_goal_override: encodePatch(request.long_term_goal_override)
  };
}
function buildPromptRollbackRequest(agentId, toVersion) {
  const profile = selectedAgentPromptProfile();
  const targetVersion = Number(toVersion);
  if (!agentId || !profile) {
    throw new Error("select an agent before rolling back prompt overrides");
  }
  if (!Number.isInteger(targetVersion) || targetVersion < 0) {
    throw new Error("prompt rollback requires integer toVersion >= 0");
  }
  return {
    agent_id: agentId,
    player_id: state.auth.playerId,
    public_key: state.auth.publicKey,
    to_version: targetVersion,
    expected_version: Number(profile.version || 0),
    updated_by: state.auth.playerId
  };
}
function pushChatHistory(entry) {
  if (!entry) {
    return;
  }
  state.chatHistory.unshift({
    id: entry.id || `${entry.source || "chat"}-${Date.now()}-${Math.random().toString(16).slice(2)}`,
    source: entry.source || "event",
    agentId: entry.agentId || null,
    locationId: entry.locationId || null,
    message: String(entry.message || ""),
    tick: Number(entry.tick || 0),
    speaker: entry.speaker || null,
    playerId: entry.playerId || null,
    targetAgentId: entry.targetAgentId || null,
    intentSeq: entry.intentSeq || null
  });
  state.chatHistory = state.chatHistory.slice(0, 40);
}
function extractAgentSpokeEntry(event) {
  const kind = event?.kind;
  const kindType = String(kind?.type || "");
  if (!["agent_spoke", "AgentSpoke"].includes(kindType)) {
    return null;
  }
  const data = kind.data || {};
  return {
    id: `event-${event.id}`,
    source: "event",
    agentId: data.agent_id || null,
    locationId: data.location_id || null,
    message: data.message || "",
    tick: Number(event.time || 0),
    speaker: data.agent_id || null,
    targetAgentId: data.target_agent_id || null
  };
}
function requestSnapshotSafe() {
  try {
    sendJson({ type: "request_snapshot" });
  } catch (_) {
  }
}
function createSemanticFeedback(kind, action, agentId, extra = {}) {
  return {
    id: nextRequestId(),
    kind,
    action,
    agentId,
    accepted: true,
    ok: false,
    stage: "queued",
    reason: null,
    effect: null,
    response: null,
    ...extra
  };
}
function markPendingSemanticRebind(message) {
  const text = String(message).trim();
  for (const feedback of [state.lastChatFeedback, state.lastPromptFeedback]) {
    if (!feedback || feedback.stage !== "registering") {
      continue;
    }
    feedback.effect = text;
    feedback.reason = null;
  }
}
function enqueueSemanticCommand(command) {
  pendingSemanticCommands.push(command);
  if (!semanticSendLoop) {
    semanticSendLoop = processSemanticCommands();
  }
}
async function processSemanticCommands() {
  try {
    while (pendingSemanticCommands.length > 0) {
      const command = pendingSemanticCommands.shift();
      try {
        await command.execute();
      } catch (error) {
        command.feedback.stage = "error";
        command.feedback.ok = false;
        command.feedback.reason = String(error);
        command.feedback.effect = "request build/send failed";
        if (command.kind === "chat") {
          state.lastChatFeedback = command.feedback;
        } else {
          state.lastPromptFeedback = command.feedback;
        }
        render();
      }
    }
  } finally {
    semanticSendLoop = null;
    if (pendingSemanticCommands.length > 0) {
      semanticSendLoop = processSemanticCommands();
    }
  }
}
function assertSemanticCapability(actionId) {
  const capability = buildSemanticCapability(actionId);
  if (!capability.enabled) {
    throw new Error(capability.reason || state.auth.error || `${actionId} is unavailable`);
  }
}
function sendAgentChat(agentIdOrPayload, maybeMessage) {
  let agentId = null;
  let message = null;
  if (typeof agentIdOrPayload === "object" && agentIdOrPayload !== null) {
    agentId = String(agentIdOrPayload.agentId || agentIdOrPayload.agent_id || selectedAgentId() || "");
    message = String(agentIdOrPayload.message || "");
  } else {
    agentId = String(agentIdOrPayload || selectedAgentId() || "");
    message = String(maybeMessage || "");
  }
  if (!agentId) {
    return { ok: false, reason: "agent chat requires a selected agent or explicit agentId" };
  }
  if (!message.trim()) {
    return { ok: false, reason: "agent chat message cannot be empty" };
  }
  const feedback = createSemanticFeedback("chat", "agent_chat", agentId, {
    effect: "queued for signing and send",
    pendingMessage: message,
    pendingPlayerId: state.auth.playerId || null
  });
  state.lastChatFeedback = feedback;
  enqueueSemanticCommand({
    kind: "chat",
    feedback,
    execute: async () => {
      await ensureHostedPlayerAuthAvailable();
      assertSemanticCapability("agent_chat");
      feedback.stage = "registering";
      feedback.effect = "registering player session";
      render();
      await ensureRegisteredPlayerSession(agentId);
      feedback.stage = "signing";
      feedback.effect = "building auth proof";
      render();
      const request = {
        agent_id: agentId,
        message,
        player_id: state.auth.playerId,
        public_key: state.auth.publicKey
      };
      request.auth = await buildAgentChatAuthProof(request, state.auth);
      feedback.stage = "sent";
      feedback.effect = "agent_chat request sent; waiting for ack";
      state.lastChatFeedback = feedback;
      sendJson({ type: "agent_chat", request });
      state.chatDraft.message = "";
      state.chatDraft.dirty = false;
      render();
    }
  });
  render();
  return { ok: true, feedback: snapshotSemanticFeedback(feedback) };
}
function sendPromptControl(mode, payload = null) {
  const normalizedMode = String(mode || "").trim().toLowerCase();
  if (!["preview", "apply", "rollback"].includes(normalizedMode)) {
    return { ok: false, reason: "prompt control mode must be preview, apply, or rollback" };
  }
  const selectedId = selectedAgentId();
  const agentId = String(payload?.agentId || payload?.agent_id || selectedId || "");
  if (!agentId) {
    return { ok: false, reason: "prompt control requires a selected agent or explicit agentId" };
  }
  let request;
  try {
    if (normalizedMode === "rollback") {
      const currentVersion = Number(state.promptDraft.currentVersion || selectedAgentPromptProfile()?.version || 0);
      const fallbackVersion = Math.max(0, currentVersion - 1);
      const toVersion = payload?.toVersion ?? payload?.to_version ?? fallbackVersion;
      request = buildPromptRollbackRequest(agentId, toVersion);
    } else {
      request = buildPromptRequestFromDraft(agentId, {
        systemPrompt: payload?.systemPrompt ?? payload?.system_prompt_override ?? state.promptDraft.systemPrompt,
        shortTermGoal: payload?.shortTermGoal ?? payload?.short_term_goal_override ?? state.promptDraft.shortTermGoal,
        longTermGoal: payload?.longTermGoal ?? payload?.long_term_goal_override ?? state.promptDraft.longTermGoal
      });
    }
  } catch (error) {
    return { ok: false, reason: String(error) };
  }
  const feedback = createSemanticFeedback("prompt", `prompt_${normalizedMode}`, agentId, {
    effect: "queued for signing and send",
    toVersion: request.to_version ?? null
  });
  state.lastPromptFeedback = feedback;
  enqueueSemanticCommand({
    kind: "prompt",
    feedback,
    execute: async () => {
      await ensureHostedPlayerAuthAvailable();
      assertSemanticCapability("prompt_control");
      feedback.stage = "registering";
      feedback.effect = "registering player session";
      render();
      await ensureRegisteredPlayerSession(agentId);
      let strongAuthGrant = null;
      if (String(state.hostedAccess?.deployment_mode || "").trim() === "hosted_public_join") {
        feedback.stage = "authorizing";
        feedback.effect = "requesting backend strong-auth grant";
        render();
        strongAuthGrant = await requestHostedStrongAuthGrant(
          normalizedMode === "rollback" ? "prompt_control_rollback" : `prompt_control_${normalizedMode}`,
          agentId
        );
      }
      feedback.stage = "signing";
      feedback.effect = "building auth proof";
      render();
      let commandRequest;
      if (normalizedMode === "rollback") {
        commandRequest = {
          ...request,
          auth: await buildPromptRollbackAuthProof(request, state.auth)
        };
        if (strongAuthGrant) {
          commandRequest.strong_auth_grant = strongAuthGrant;
        }
      } else {
        commandRequest = encodePromptRequestForJson(request);
        commandRequest.auth = await buildPromptControlAuthProof(normalizedMode, request, state.auth);
        if (strongAuthGrant) {
          commandRequest.strong_auth_grant = strongAuthGrant;
        }
      }
      feedback.stage = "sent";
      feedback.effect = `prompt ${normalizedMode} request sent; waiting for ack`;
      state.lastPromptFeedback = feedback;
      sendJson({
        type: "prompt_control",
        command: {
          mode: normalizedMode,
          request: commandRequest
        }
      });
      render();
    }
  });
  render();
  return { ok: true, feedback: snapshotSemanticFeedback(feedback) };
}
function applyPromptAckLocally(ack) {
  const agentId = ack?.agent_id;
  if (!agentId || !state.snapshot?.model) {
    return;
  }
  if (!state.snapshot.model.agent_prompt_profiles) {
    state.snapshot.model.agent_prompt_profiles = {};
  }
  const current = state.snapshot.model.agent_prompt_profiles[agentId] || { agent_id: agentId };
  const nextProfile = {
    ...current,
    agent_id: agentId,
    version: Number(ack.version || current.version || 0),
    updated_at_tick: Number(ack.updated_at_tick || state.logicalTime),
    updated_by: state.auth.playerId || current.updated_by || ""
  };
  if (!ack.preview) {
    nextProfile.system_prompt_override = state.promptDraft.systemPrompt || null;
    nextProfile.short_term_goal_override = state.promptDraft.shortTermGoal || null;
    nextProfile.long_term_goal_override = state.promptDraft.longTermGoal || null;
  }
  state.snapshot.model.agent_prompt_profiles[agentId] = nextProfile;
  if (selectedAgentId() === agentId) {
    state.promptDraft = {
      agentId,
      currentVersion: nextProfile.version,
      rollbackTargetVersion: Math.max(0, Number(nextProfile.version || 0) - 1),
      updatedBy: nextProfile.updated_by,
      updatedAtTick: nextProfile.updated_at_tick,
      systemPrompt: String(nextProfile.system_prompt_override || ""),
      shortTermGoal: String(nextProfile.short_term_goal_override || ""),
      longTermGoal: String(nextProfile.long_term_goal_override || ""),
      dirty: false
    };
  }
}
function handlePromptControlAck(ack) {
  const feedback = state.lastPromptFeedback || createSemanticFeedback("prompt", "prompt_ack", ack?.agent_id || null);
  const operation = String(ack?.operation || (ack?.preview ? "preview" : "apply"));
  feedback.stage = ack?.preview ? "preview_ack" : operation === "rollback" ? "rollback_ack" : "apply_ack";
  feedback.ok = true;
  feedback.accepted = true;
  feedback.reason = null;
  feedback.effect = ack?.preview ? `prompt preview ready: version=${ack.version}` : operation === "rollback" ? `prompt rolled back via version=${ack.version} → target=${Number(ack?.rolled_back_to_version || 0)}` : `prompt applied: version=${ack.version}`;
  feedback.response = clone(ack);
  state.lastPromptFeedback = feedback;
  if (ack?.preview) {
    return;
  }
  if (operation === "rollback") {
    state.promptDraft.currentVersion = Number(ack?.version || state.promptDraft.currentVersion || 0);
    state.promptDraft.rollbackTargetVersion = Math.max(0, state.promptDraft.currentVersion - 1);
    state.promptDraft.dirty = false;
    requestSnapshotSafe();
    return;
  }
  applyPromptAckLocally(ack);
}
function handlePromptControlError(error) {
  const feedback = state.lastPromptFeedback || createSemanticFeedback("prompt", "prompt_error", error?.agent_id || selectedAgentId());
  feedback.stage = "error";
  feedback.ok = false;
  feedback.accepted = false;
  feedback.reason = error?.message || error?.code || "prompt control failed";
  feedback.effect = error?.code || "prompt control error";
  feedback.response = clone(error);
  state.lastPromptFeedback = feedback;
}
function handleAgentChatAck(ack) {
  const feedback = state.lastChatFeedback || createSemanticFeedback("chat", "agent_chat", ack?.agent_id || null);
  feedback.stage = "ack";
  feedback.ok = true;
  feedback.accepted = true;
  feedback.reason = null;
  feedback.effect = `chat accepted at tick ${Number(ack?.accepted_at_tick || state.logicalTime)}`;
  feedback.response = clone(ack);
  state.lastChatFeedback = feedback;
  pushChatHistory({
    id: `chat-ack-${feedback.id}`,
    source: "player",
    agentId: ack?.agent_id || feedback.agentId || null,
    message: feedback.pendingMessage || "",
    tick: Number(ack?.accepted_at_tick || state.logicalTime || 0),
    speaker: feedback.pendingPlayerId || state.auth.playerId || null,
    playerId: feedback.pendingPlayerId || state.auth.playerId || null,
    targetAgentId: ack?.agent_id || feedback.agentId || null,
    intentSeq: ack?.intent_seq || null
  });
}
function handleAgentChatError(error) {
  const feedback = state.lastChatFeedback || createSemanticFeedback("chat", "agent_chat", error?.agent_id || selectedAgentId());
  feedback.stage = "error";
  feedback.ok = false;
  feedback.accepted = false;
  feedback.reason = error?.message || error?.code || "agent chat failed";
  feedback.effect = error?.code || "agent chat error";
  feedback.response = clone(error);
  state.lastChatFeedback = feedback;
}
function adoptHostedRecoveryAck(ack) {
  if (!ack || !state.auth.available || state.auth.source === "legacy_viewer_auth_bootstrap") {
    return;
  }
  const hadPendingForceRebind = state.auth.pendingForceRebind === true;
  const previousRequestedAgentId = state.auth.pendingRequestedAgentId;
  state.auth.syncInFlight = false;
  state.auth.recoveryErrorCode = null;
  state.auth.recoveryErrorMessage = null;
  state.auth.error = null;
  state.auth.revokeReason = null;
  state.auth.revokedBy = null;
  if (ack.player_id) {
    state.auth.playerId = ack.player_id;
  }
  if (ack.session_pubkey) {
    state.auth.publicKey = ack.session_pubkey;
  }
  if (ack.session_epoch != null) {
    state.auth.sessionEpoch = Number(ack.session_epoch);
  }
  state.auth.boundAgentId = ack.agent_id || null;
  state.auth.pendingRequestedAgentId = ack.agent_id || state.auth.pendingRequestedAgentId || null;
  state.auth.pendingForceRebind = false;
  if (ack.status === "session_registered" && hadPendingForceRebind) {
    state.auth.rebindNotice = `Player session switched to ${ack.agent_id || previousRequestedAgentId || "requested agent"}.`;
  }
  state.auth.registrationStatus = ack.status === "session_registered" || ack.status === "catch_up_ready" ? "registered" : ack.status === "session_revoked" ? "guest" : "issued";
  state.auth.runtimeStatus = ack.status === "session_revoked" ? "revoked" : ack.agent_id ? "registered" : "registered_unbound";
  if (ack.status === "session_revoked") {
    void releaseHostedPlayerSlot().catch(() => {
    });
    resetHostedPlayerAuthState(
      ack.message || "hosted player session was revoked",
      {
        revokeReason: ack.revoke_reason || ack.message || null,
        revokedBy: ack.revoked_by || null
      }
    );
  } else {
    persistHostedPlayerSession(state.auth);
    void refreshHostedPlayerLease();
    syncHostedSessionRefreshLoop();
  }
  if (pendingSessionRegisterWaiter && ack.status === "session_registered") {
    const waiter = pendingSessionRegisterWaiter;
    pendingSessionRegisterWaiter = null;
    waiter.resolve(ack);
  }
}
async function recoverHostedSessionFromError(error) {
  if (!canAutoIssueHostedPlayerSession() || state.auth.source === "legacy_viewer_auth_bootstrap") {
    return;
  }
  const code = String(error?.code || "").trim();
  if (recoveryErrorRequiresExplicitRebind(error) && state.auth.pendingRequestedAgentId && !state.auth.pendingForceRebind) {
    await ensureRegisteredPlayerSession(state.auth.pendingRequestedAgentId, { forceRebind: true });
    return;
  }
  if (code === "session_not_found") {
    await ensureRegisteredPlayerSession(latestRequestedAgentId());
    return;
  }
  if (code === "session_revoked") {
    void releaseHostedPlayerSlot().catch(() => {
    });
    resetHostedPlayerAuthState(
      error?.message || code || "hosted player session failed",
      {
        revokeReason: error?.revoke_reason || error?.message || null,
        revokedBy: error?.revoked_by || null
      }
    );
    render();
    return;
  }
  if (["session_key_mismatch", "session_player_id_invalid"].includes(code)) {
    void releaseHostedPlayerSlot().catch(() => {
    });
    resetHostedPlayerAuthState(error?.message || code || "hosted player session failed");
    render();
    await issueHostedPlayerIdentity();
    if (state.auth.available) {
      await ensureRegisteredPlayerSession(latestRequestedAgentId());
    }
  }
}
function handleAuthoritativeRecoveryAck(ack) {
  adoptHostedRecoveryAck(ack);
}
function handleAuthoritativeRecoveryError(error) {
  if (pendingSessionRegisterWaiter && recoveryErrorRequiresExplicitRebind(error) && pendingSessionRegisterWaiter.requestedAgentId && !pendingSessionRegisterWaiter.forceRebind) {
    state.auth.recoveryErrorCode = error?.code || null;
    state.auth.recoveryErrorMessage = error?.message || null;
    state.auth.error = error?.message || error?.code || "authoritative recovery failed";
    state.auth.registrationStatus = "registering";
    state.auth.runtimeStatus = "rebind_retrying";
    state.auth.pendingForceRebind = true;
    state.auth.rebindNotice = `Requested agent ${state.auth.pendingRequestedAgentId || "-"} needs explicit rebind; retrying now.`;
    markPendingSemanticRebind("explicit rebind required; retrying registration for the requested agent");
    render();
    void retryPendingSessionRegisterWaiterWithForceRebind().catch((retryError) => {
      handleAuthoritativeRecoveryError({
        code: "player_bind_failed",
        message: String(retryError)
      });
    });
    return;
  }
  if (!state.auth.available || state.auth.source === "legacy_viewer_auth_bootstrap") {
    clearPendingSessionRegisterWaiter(error?.message || error?.code || "authoritative recovery failed");
    return;
  }
  state.auth.syncInFlight = false;
  state.auth.recoveryErrorCode = error?.code || null;
  state.auth.recoveryErrorMessage = error?.message || null;
  state.auth.error = error?.message || error?.code || "authoritative recovery failed";
  state.auth.revokeReason = error?.revoke_reason || null;
  state.auth.revokedBy = error?.revoked_by || null;
  state.auth.registrationStatus = "issued";
  state.auth.runtimeStatus = error?.code === "session_revoked" ? "revoked" : error?.code === "session_not_found" ? "missing" : "error";
  if (!recoveryErrorRequiresExplicitRebind(error)) {
    state.auth.boundAgentId = null;
  }
  clearPendingSessionRegisterWaiter(error?.message || error?.code || "authoritative recovery failed");
  syncHostedSessionRefreshLoop();
  void recoverHostedSessionFromError(error);
}
function handleViewerMessage(message) {
  switch (message?.type) {
    case "hello_ack":
      state.server = message.server || null;
      state.worldId = message.world_id || null;
      state.controlProfile = message.control_profile || "playback";
      state.debugViewerStatus = "subscribed";
      void ensureHostedPlayerAuthAvailable().then(() => {
        syncHostedPlayerSessionOnConnect();
        render();
      });
      break;
    case "snapshot":
      handleSnapshot(message.snapshot);
      break;
    case "event": {
      addRecentEvent(message.event);
      const chatEntry = extractAgentSpokeEntry(message.event);
      if (chatEntry) {
        pushChatHistory(chatEntry);
      }
      state.logicalTime = Math.max(state.logicalTime, Number(message.event?.time || 0));
      state.tick = state.logicalTime;
      break;
    }
    case "metrics":
      handleMetrics(message.time, message.metrics);
      break;
    case "control_completion_ack":
      handleControlCompletionAck(message.ack);
      break;
    case "prompt_control_ack":
      handlePromptControlAck(message.ack);
      break;
    case "prompt_control_error":
      handlePromptControlError(message.error);
      break;
    case "agent_chat_ack":
      handleAgentChatAck(message.ack);
      break;
    case "agent_chat_error":
      handleAgentChatError(message.error);
      break;
    case "authoritative_recovery_ack":
      handleAuthoritativeRecoveryAck(message.ack);
      break;
    case "authoritative_recovery_error":
      handleAuthoritativeRecoveryError(message.error);
      break;
    case "error":
      reportFatalError(message.message, "viewer");
      break;
  }
  updateControlFeedbackFromProgress();
  render();
}
function attachSocket(ws) {
  ws.addEventListener("open", () => {
    state.connectionStatus = "connected";
    state.debugViewerStatus = "detached";
    state.lastError = null;
    sendJson({ type: "hello", client: "software_safe_viewer", version: 1 });
    sendJson({ type: "subscribe", streams: ["snapshot", "events", "metrics"], event_kinds: [] });
    sendJson({ type: "request_snapshot" });
    syncHostedSessionRefreshLoop();
    render();
  });
  ws.addEventListener("message", (event) => {
    try {
      const message = JSON.parse(String(event.data || "null"));
      handleViewerMessage(message);
    } catch (error) {
      reportFatalError(String(error), "viewer.parse");
    }
  });
  ws.addEventListener("error", () => {
    reportFatalError("websocket error", "viewer.ws");
  });
  ws.addEventListener("close", () => {
    state.connectionStatus = "connecting";
    state.debugViewerStatus = "detached";
    if (state.auth.available && state.auth.source !== "legacy_viewer_auth_bootstrap") {
      state.auth.syncInFlight = false;
      state.auth.runtimeStatus = "disconnected";
    }
    clearPendingSessionRegisterWaiter("websocket disconnected during player session registration");
    stopHostedSessionRefreshLoop();
    render();
    if (reconnectTimer) {
      window.clearTimeout(reconnectTimer);
    }
    reconnectTimer = window.setTimeout(connect, 1200);
  });
}
function connect() {
  if (socket) {
    try {
      socket.close();
    } catch (_) {
    }
  }
  const params = getSearchParams();
  state.wsUrl = normalizeWsAddr(params.get("ws") || params.get("addr") || DEFAULT_WS_ADDR);
  state.connectionStatus = "connecting";
  render();
  socket = new WebSocket(state.wsUrl);
  attachSocket(socket);
}
function resourceSummary(resources) {
  if (!resources || typeof resources !== "object") {
    return "-";
  }
  return Object.entries(resources).map(([key, value]) => {
    if (value && typeof value === "object") {
      return `${key}:${JSON.stringify(value)}`;
    }
    return `${key}:${value}`;
  }).join(" · ") || "-";
}
function modelLists() {
  const { agents, locations } = entityCollections();
  const keyword = selectedSearch.trim().toLowerCase();
  const filter = (entry, label) => {
    if (!keyword) return true;
    return String(label).toLowerCase().includes(keyword);
  };
  return {
    agents: agents.filter((agent) => filter(agent, `${agent.id} ${agent.location_id}`)).sort((a, b) => String(a.id).localeCompare(String(b.id))),
    locations: locations.filter((location) => filter(location, `${location.id} ${location.name}`)).sort((a, b) => String(a.id).localeCompare(String(b.id)))
  };
}
function connectionBadgeClass() {
  if (state.connectionStatus === "connected") return "badge badge--good";
  if (state.connectionStatus === "error") return "badge badge--bad";
  return "badge badge--warn";
}
function feedbackBadgeClass(feedback) {
  if (!feedback) return "badge";
  if (feedback.stage === "error") return "badge badge--bad";
  if (feedback.ok) return "badge badge--good";
  return "badge badge--warn";
}
function render() {
  renderHook();
}
function setStrongAuthApprovalCode(value) {
  state.strongAuth.approvalCode = String(value || "");
  render();
  return {
    ok: true,
    configured: !!state.strongAuth.approvalCode.trim()
  };
}
function installTestApi() {
  window[TEST_API_GLOBAL_NAME] = {
    getState,
    describeControls,
    fillControlExample,
    sendControl,
    runSteps,
    setMode,
    focus,
    select,
    sendAgentChat,
    sendPromptControl,
    setStrongAuthApprovalCode,
    logoutHostedPlayerSession,
    retryHostedPlayerIdentityIssue,
    reportFatalError
  };
}
function bootstrap() {
  Object.assign(state, detectRendererMeta());
  state.hostedAccess = resolveHostedAccessHint();
  state.auth = resolveViewerAuthState();
  state.wsUrl = initialWsUrl();
  window[RENDER_META_GLOBAL_NAME] = Object.freeze({
    renderMode: state.renderMode,
    rendererClass: state.rendererClass,
    softwareSafeReason: state.softwareSafeReason,
    renderer: state.renderer,
    vendor: state.vendor,
    webglVersion: state.webglVersion
  });
  installTestApi();
  render();
  void refreshHostedAdmissionState().then(() => render());
  void ensureHostedPlayerAuthAvailable().then(() => render());
  connect();
}
function initializeSoftwareSafeCore() {
  if (bootstrapped) {
    return;
  }
  bootstrapped = true;
  bootstrap();
}
window.addEventListener("error", (event) => {
  const message = event?.message || event?.error?.message || "window error";
  reportFatalError(message, "window.error");
});
window.addEventListener("unhandledrejection", (event) => {
  const message = event?.reason?.message || String(event?.reason || "unhandled rejection");
  reportFatalError(message, "window.unhandledrejection");
});
var _tmpl$ = /* @__PURE__ */ template(`<span>`), _tmpl$2 = /* @__PURE__ */ template(`<div class=empty>`), _tmpl$3 = /* @__PURE__ */ template(`<pre class=json>`), _tmpl$4 = /* @__PURE__ */ template(`<div class=badge-row style=margin-top:8px>`), _tmpl$5 = /* @__PURE__ */ template(`<div class=metric><div class=metric__label></div><div class=metric__value>`), _tmpl$6 = /* @__PURE__ */ template(`<div class=event-card__meta>`), _tmpl$7 = /* @__PURE__ */ template(`<div class=event-card><div class=event-card__title><span>`), _tmpl$8 = /* @__PURE__ */ template(`<div class="panel panel--nested"style=background:rgba(255,255,255,0.02)><div class=panel__header><div class=panel__title></div></div><div class="panel__body stack">`), _tmpl$9 = /* @__PURE__ */ template(`<div class=stack><div class=field><label for=entity-search>Filter targets</label><input id=entity-search type=search placeholder="Search agents or locations"></div><div><div class=panel__title style=margin-bottom:10px>Agents</div><div class=list></div></div><div><div class=panel__title style=margin-bottom:10px>Locations</div><div class=list>`), _tmpl$0 = /* @__PURE__ */ template(`<button class=list-item data-select-kind=agent><div class=list-item__title></div><div class=list-item__meta>`), _tmpl$1 = /* @__PURE__ */ template(`<button class=list-item data-select-kind=location><div class=list-item__title></div><div class=list-item__meta>`), _tmpl$10 = /* @__PURE__ */ template(`<div class=badge-row>`), _tmpl$11 = /* @__PURE__ */ template(`<div class=toolbar><button data-auth-action=retry-issue>Acquire Hosted Player Session`), _tmpl$12 = /* @__PURE__ */ template(`<div class=toolbar><button data-auth-action=logout>Release Hosted Player Session`), _tmpl$13 = /* @__PURE__ */ template(`<div class=event-list>`), _tmpl$14 = /* @__PURE__ */ template(`<div class=stack><div class=badge-row></div><div class=summary-grid></div><div class=badge-row></div><div class=badge-row></div><div class=summary-grid></div><div><div class=panel__title style=margin-bottom:10px>Recent Events</div><div class=event-list>`), _tmpl$15 = /* @__PURE__ */ template(`<div class="panel panel--nested"style=background:rgba(255,255,255,0.02);border-color:rgba(255,184,77,0.35)><div class=panel__header><div class=panel__title>Hosted Recovery</div></div><div class="panel__body stack"><div class=badge-row></div><div class=toolbar><button data-auth-action=retry-issue>`), _tmpl$16 = /* @__PURE__ */ template(`<div class=toolbar><button data-action=play>Play</button><button data-action=pause>Pause</button><button data-action=step>Step x1`), _tmpl$17 = /* @__PURE__ */ template(`<div class=control-grid><div class=field><label for=step-count>Step count</label><input id=step-count type=number min=1 step=1></div><div class=field style=align-self:end><button data-action=step-count>Step custom count`), _tmpl$18 = /* @__PURE__ */ template(`<div class=field><label for=strong-auth-approval-code>Backend Approval Code</label><input id=strong-auth-approval-code type=password autocomplete=off>`), _tmpl$19 = /* @__PURE__ */ template(`<div class=field><label for=prompt-system>System Prompt Override</label><textarea id=prompt-system rows=4>`), _tmpl$20 = /* @__PURE__ */ template(`<div class=field><label for=prompt-short>Short-Term Goal Override</label><textarea id=prompt-short rows=3>`), _tmpl$21 = /* @__PURE__ */ template(`<div class=field><label for=prompt-long>Long-Term Goal Override</label><textarea id=prompt-long rows=3>`), _tmpl$22 = /* @__PURE__ */ template(`<div class=toolbar><button data-prompt-action=preview>Preview Prompt</button><button data-prompt-action=apply>Apply Prompt`), _tmpl$23 = /* @__PURE__ */ template(`<div class=toolbar><div class=field style=margin:0;min-width:180px;flex:1><label for=prompt-rollback-version>Rollback Target Version</label><input id=prompt-rollback-version type=number min=0 step=1></div><button data-prompt-action=rollback>Rollback Prompt`), _tmpl$24 = /* @__PURE__ */ template(`<div class=toolbar><button disabled>Main Token Transfer (Not Exposed Here Yet)`), _tmpl$25 = /* @__PURE__ */ template(`<div class=field><label for=agent-chat-message>Message</label><textarea id=agent-chat-message rows=4 placeholder="Send a message to the selected agent">`), _tmpl$26 = /* @__PURE__ */ template(`<div class=toolbar><button data-chat-send=1>Send Chat`), _tmpl$27 = /* @__PURE__ */ template(`<div><div class=panel__title style=margin-bottom:10px>Message Flow</div><div class=event-list>`), _tmpl$28 = /* @__PURE__ */ template(`<div class=stack><div class=badge-row></div><div class=badge-row>`), _tmpl$29 = /* @__PURE__ */ template(`<div><div class=panel__title style=margin-bottom:10px;color:var(--bad)>Last Error</div><pre class=json>`), _tmpl$30 = /* @__PURE__ */ template(`<div class=stack><div class=badge-row></div><div><div class=panel__title style=margin-bottom:10px>Snapshot Summary`), _tmpl$31 = /* @__PURE__ */ template(`<section class=panel><div class=panel__header><div class=panel__title>Targets</div></div><div class=panel__body>`), _tmpl$32 = /* @__PURE__ */ template(`<section class=panel><div class=panel__header><div class=panel__title>World Summary</div></div><div class=panel__body>`), _tmpl$33 = /* @__PURE__ */ template(`<section class=panel><div class=panel__header><div class=panel__title>Details</div></div><div class=panel__body>`);
function Badge(props) {
  return (() => {
    var _el$ = _tmpl$();
    insert(_el$, () => props.children);
    createRenderEffect(() => className(_el$, props.class ?? "badge"));
    return _el$;
  })();
}
function EmptyState(props) {
  return (() => {
    var _el$2 = _tmpl$2();
    insert(_el$2, () => props.children);
    createRenderEffect((_$p) => style(_el$2, props.style, _$p));
    return _el$2;
  })();
}
function JsonBlock(props) {
  return (() => {
    var _el$3 = _tmpl$3();
    insert(_el$3, () => JSON.stringify(props.value, null, 2));
    return _el$3;
  })();
}
function MetricCard(props) {
  return (() => {
    var _el$4 = _tmpl$5(), _el$5 = _el$4.firstChild, _el$6 = _el$5.nextSibling;
    insert(_el$5, () => props.label);
    insert(_el$6, () => props.value);
    insert(_el$4, createComponent(Show, {
      get when() {
        return props.children;
      },
      get children() {
        var _el$7 = _tmpl$4();
        insert(_el$7, () => props.children);
        return _el$7;
      }
    }), null);
    return _el$4;
  })();
}
function EventCard(props) {
  return (() => {
    var _el$8 = _tmpl$7(), _el$9 = _el$8.firstChild, _el$0 = _el$9.firstChild;
    insert(_el$0, () => props.title);
    insert(_el$9, createComponent(Show, {
      get when() {
        return props.badge;
      },
      get children() {
        var _el$1 = _tmpl$();
        insert(_el$1, () => props.badge);
        createRenderEffect(() => className(_el$1, props.badgeClass ?? "badge"));
        return _el$1;
      }
    }), null);
    insert(_el$8, createComponent(Show, {
      get when() {
        return props.meta;
      },
      get children() {
        var _el$10 = _tmpl$6();
        insert(_el$10, () => props.meta);
        return _el$10;
      }
    }), null);
    insert(_el$8, () => props.children, null);
    return _el$8;
  })();
}
function PanelSection(props) {
  return (() => {
    var _el$11 = _tmpl$8(), _el$12 = _el$11.firstChild, _el$13 = _el$12.firstChild, _el$14 = _el$12.nextSibling;
    insert(_el$13, () => props.title);
    insert(_el$14, () => props.children);
    return _el$11;
  })();
}
function renderResourceSummary(resources) {
  return resourceSummary(resources);
}
function TargetsPanel() {
  const lists = () => modelLists();
  return (() => {
    var _el$15 = _tmpl$9(), _el$16 = _el$15.firstChild, _el$17 = _el$16.firstChild, _el$18 = _el$17.nextSibling, _el$19 = _el$16.nextSibling, _el$20 = _el$19.firstChild, _el$21 = _el$20.nextSibling, _el$22 = _el$19.nextSibling, _el$23 = _el$22.firstChild, _el$24 = _el$23.nextSibling;
    _el$18.$$input = (event) => setSelectedSearch(event.currentTarget.value);
    insert(_el$21, createComponent(Show, {
      get when() {
        return lists().agents.length > 0;
      },
      get fallback() {
        return createComponent(EmptyState, {
          children: "No agents in current snapshot."
        });
      },
      get children() {
        return createComponent(For, {
          get each() {
            return lists().agents;
          },
          children: (agent) => (() => {
            var _el$25 = _tmpl$0(), _el$26 = _el$25.firstChild, _el$27 = _el$26.nextSibling;
            _el$25.$$click = () => applySelection({
              kind: "agent",
              id: agent.id
            });
            insert(_el$26, () => agent.id);
            insert(_el$27, () => `location=${agent.location_id} · resources=${renderResourceSummary(agent.resources)}`);
            createRenderEffect((_p$) => {
              var _v$ = agent.id, _v$2 = state.selectedKind === "agent" && state.selectedId === agent.id;
              _v$ !== _p$.e && setAttribute(_el$25, "data-select-id", _p$.e = _v$);
              _v$2 !== _p$.t && setAttribute(_el$25, "data-selected", _p$.t = _v$2);
              return _p$;
            }, {
              e: void 0,
              t: void 0
            });
            return _el$25;
          })()
        });
      }
    }));
    insert(_el$24, createComponent(Show, {
      get when() {
        return lists().locations.length > 0;
      },
      get fallback() {
        return createComponent(EmptyState, {
          children: "No locations in current snapshot."
        });
      },
      get children() {
        return createComponent(For, {
          get each() {
            return lists().locations;
          },
          children: (location) => (() => {
            var _el$28 = _tmpl$1(), _el$29 = _el$28.firstChild, _el$30 = _el$29.nextSibling;
            _el$28.$$click = () => applySelection({
              kind: "location",
              id: location.id
            });
            insert(_el$29, () => location.name || location.id);
            insert(_el$30, () => `id=${location.id} · resources=${renderResourceSummary(location.resources)}`);
            createRenderEffect((_p$) => {
              var _v$3 = location.id, _v$4 = state.selectedKind === "location" && state.selectedId === location.id;
              _v$3 !== _p$.e && setAttribute(_el$28, "data-select-id", _p$.e = _v$3);
              _v$4 !== _p$.t && setAttribute(_el$28, "data-selected", _p$.t = _v$4);
              return _p$;
            }, {
              e: void 0,
              t: void 0
            });
            return _el$28;
          })()
        });
      }
    }));
    createRenderEffect(() => _el$18.value = getSelectedSearch());
    return _el$15;
  })();
}
function WorldSummaryPanel() {
  const state$1 = state;
  const controlFeedback = () => snapshotControlFeedback(state$1.lastControlFeedback);
  const promptFeedback = () => snapshotSemanticFeedback(state$1.lastPromptFeedback);
  const chatFeedback = () => snapshotSemanticFeedback(state$1.lastChatFeedback);
  const authSurface = () => buildAuthSurfaceModel();
  const hostedActionMatrixView = () => buildHostedActionMatrixView();
  const hostedRecoveryHint = () => buildHostedRecoveryHint();
  const selectedDebug = () => selectedAgentExecutionDebugContext();
  const tierBadgeClass = (status) => status === "active" || status === "active_legacy_preview" ? "badge badge--good" : status === "superseded" ? "badge" : "badge badge--warn";
  const showRebindNotice = () => Boolean(state$1.auth.pendingRequestedAgentId) && (state$1.auth.pendingForceRebind || state$1.auth.runtimeStatus === "rebind_retrying" || state$1.auth.runtimeStatus === "rebind_registering");
  return (() => {
    var _el$31 = _tmpl$14(), _el$32 = _el$31.firstChild, _el$33 = _el$32.nextSibling, _el$34 = _el$33.nextSibling, _el$36 = _el$34.nextSibling, _el$47 = _el$36.nextSibling, _el$48 = _el$47.nextSibling, _el$49 = _el$48.firstChild, _el$50 = _el$49.nextSibling;
    insert(_el$32, createComponent(Badge, {
      "class": "badge badge--accent",
      children: "software_safe"
    }), null);
    insert(_el$32, createComponent(Badge, {
      get ["class"]() {
        return connectionBadgeClass();
      },
      get children() {
        return state$1.connectionStatus;
      }
    }), null);
    insert(_el$32, createComponent(Badge, {
      get children() {
        return `debugViewer=${state$1.debugViewerMode}:${state$1.debugViewerStatus}`;
      }
    }), null);
    insert(_el$32, createComponent(Badge, {
      get children() {
        return `rendererClass=${state$1.rendererClass}`;
      }
    }), null);
    insert(_el$32, createComponent(Badge, {
      get children() {
        return `controlProfile=${state$1.controlProfile}`;
      }
    }), null);
    insert(_el$33, createComponent(MetricCard, {
      label: "Logical Time",
      get value() {
        return state$1.logicalTime;
      }
    }), null);
    insert(_el$33, createComponent(MetricCard, {
      label: "Event Seq",
      get value() {
        return state$1.eventSeq;
      }
    }), null);
    insert(_el$33, createComponent(MetricCard, {
      label: "World",
      get value() {
        return state$1.worldId || "-";
      }
    }), null);
    insert(_el$33, createComponent(MetricCard, {
      label: "Viewer Server",
      get value() {
        return state$1.server || "-";
      }
    }), null);
    insert(_el$34, createComponent(Badge, {
      get children() {
        return `ws=${state$1.wsUrl || "-"}`;
      }
    }), null);
    insert(_el$34, createComponent(Badge, {
      get children() {
        return `reason=${state$1.softwareSafeReason || "-"}`;
      }
    }), null);
    insert(_el$34, createComponent(Badge, {
      get children() {
        return `renderer=${state$1.renderer || "n/a"}`;
      }
    }), null);
    insert(_el$31, createComponent(PanelSection, {
      title: "Execution Lanes",
      get children() {
        return [(() => {
          var _el$35 = _tmpl$10();
          insert(_el$35, createComponent(Badge, {
            "class": "badge badge--accent",
            children: "debug_viewer"
          }), null);
          insert(_el$35, createComponent(Badge, {
            get children() {
              return `status=${state$1.debugViewerStatus}`;
            }
          }), null);
          insert(_el$35, createComponent(Badge, {
            get children() {
              return `renderMode=${state$1.renderMode}`;
            }
          }), null);
          insert(_el$35, createComponent(Badge, {
            get children() {
              return `fallback=${state$1.softwareSafeReason || "-"}`;
            }
          }), null);
          return _el$35;
        })(), createComponent(EmptyState, {
          style: "margin-top:-2px;",
          children: "debug_viewer is a read-only subscription lane for runtime snapshots/events; closing the viewer does not stop the agent lane."
        }), createComponent(Show, {
          get when() {
            return selectedDebug();
          },
          get fallback() {
            return createComponent(EmptyState, {
              children: "Select an agent to compare the headless execution lane against this debug_viewer observer lane."
            });
          },
          children: (debug) => [(() => {
            var _el$51 = _tmpl$10();
            insert(_el$51, createComponent(Badge, {
              "class": "badge badge--accent",
              children: "selected agent lane"
            }), null);
            insert(_el$51, createComponent(Badge, {
              get children() {
                return `provider=${debug().provider_mode || "-"}`;
              }
            }), null);
            insert(_el$51, createComponent(Badge, {
              get children() {
                return `mode=${debug().execution_mode || "-"}`;
              }
            }), null);
            insert(_el$51, createComponent(Badge, {
              get children() {
                return `env=${debug().environment_class || "-"}`;
              }
            }), null);
            return _el$51;
          })(), (() => {
            var _el$52 = _tmpl$10();
            insert(_el$52, createComponent(Badge, {
              get children() {
                return `obs=${debug().observation_schema_version || "-"}`;
              }
            }), null);
            insert(_el$52, createComponent(Badge, {
              get children() {
                return `act=${debug().action_schema_version || "-"}`;
              }
            }), null);
            insert(_el$52, createComponent(Badge, {
              get children() {
                return `agentProfile=${debug().agent_profile || "-"}`;
              }
            }), null);
            insert(_el$52, createComponent(Badge, {
              get children() {
                return `fallback=${debug().fallback_reason || "-"}`;
              }
            }), null);
            return _el$52;
          })(), createComponent(JsonBlock, {
            get value() {
              return debug();
            }
          })]
        })];
      }
    }), _el$36);
    insert(_el$36, createComponent(Badge, {
      get ["class"]() {
        return state$1.auth.available ? "badge badge--good" : "badge badge--warn";
      },
      get children() {
        return `auth=${state$1.auth.available ? state$1.auth.registrationStatus || "ready" : "missing"}`;
      }
    }), null);
    insert(_el$36, createComponent(Badge, {
      "class": "badge badge--accent",
      get children() {
        return `tier=${authSurface().currentTier}`;
      }
    }), null);
    insert(_el$36, createComponent(Badge, {
      get children() {
        return `source=${authSurface().source}`;
      }
    }), null);
    insert(_el$36, createComponent(Badge, {
      get children() {
        return `deploymentHint=${authSurface().deploymentHint}`;
      }
    }), null);
    insert(_el$36, createComponent(Badge, {
      get children() {
        return `player=${state$1.auth.playerId || "-"}`;
      }
    }), null);
    insert(_el$36, createComponent(Badge, {
      get children() {
        return `pubkey=${state$1.auth.publicKey ? `${state$1.auth.publicKey.slice(0, 10)}…` : "-"}`;
      }
    }), null);
    insert(_el$36, createComponent(Badge, {
      get children() {
        return `epoch=${state$1.auth.sessionEpoch == null ? "-" : state$1.auth.sessionEpoch}`;
      }
    }), null);
    insert(_el$36, createComponent(Badge, {
      get children() {
        return `runtime=${state$1.auth.runtimeStatus || "-"}`;
      }
    }), null);
    insert(_el$36, createComponent(Badge, {
      get children() {
        return `boundAgent=${state$1.auth.boundAgentId || "-"}`;
      }
    }), null);
    insert(_el$36, createComponent(Badge, {
      get children() {
        return `requestedAgent=${state$1.auth.pendingRequestedAgentId || "-"}`;
      }
    }), null);
    insert(_el$36, createComponent(Badge, {
      get children() {
        return state$1.auth.pendingForceRebind ? "rebind=forcing" : "rebind=idle";
      }
    }), null);
    insert(_el$31, createComponent(Show, {
      get when() {
        return state$1.auth.recoveryErrorCode || state$1.auth.recoveryErrorMessage;
      },
      get children() {
        var _el$37 = _tmpl$10();
        insert(_el$37, createComponent(Badge, {
          "class": "badge badge--warn",
          get children() {
            return `recoveryError=${state$1.auth.recoveryErrorCode || "-"}`;
          }
        }), null);
        insert(_el$37, createComponent(Badge, {
          get children() {
            return state$1.auth.recoveryErrorMessage || "-";
          }
        }), null);
        return _el$37;
      }
    }), _el$47);
    insert(_el$31, createComponent(Show, {
      get when() {
        return showRebindNotice();
      },
      get children() {
        return [(() => {
          var _el$38 = _tmpl$10();
          insert(_el$38, createComponent(Badge, {
            "class": "badge badge--accent",
            children: "rebind"
          }), null);
          insert(_el$38, createComponent(Badge, {
            get children() {
              return `target=${state$1.auth.pendingRequestedAgentId || "-"}`;
            }
          }), null);
          insert(_el$38, createComponent(Badge, {
            get children() {
              return state$1.auth.pendingForceRebind ? "mode=force_rebind" : "mode=awaiting_retry";
            }
          }), null);
          return _el$38;
        })(), createComponent(EmptyState, {
          children: "Player session is switching to the requested agent and the current action will continue after registration succeeds."
        })];
      }
    }), _el$47);
    insert(_el$31, createComponent(Show, {
      get when() {
        return state$1.auth.rebindNotice;
      },
      get children() {
        return createComponent(EmptyState, {
          get children() {
            return state$1.auth.rebindNotice;
          }
        });
      }
    }), _el$47);
    insert(_el$31, createComponent(Show, {
      get when() {
        return state$1.hostedAdmission;
      },
      children: (admission) => (() => {
        var _el$53 = _tmpl$10();
        insert(_el$53, createComponent(Badge, {
          get children() {
            return `activeSlots=${admission().active_player_sessions}/${admission().max_player_sessions}`;
          }
        }), null);
        insert(_el$53, createComponent(Badge, {
          get children() {
            return `effectiveSlots=${admission().effective_player_sessions == null ? "-" : `${admission().effective_player_sessions}/${admission().max_player_sessions}`}`;
          }
        }), null);
        insert(_el$53, createComponent(Badge, {
          get children() {
            return `runtimeBound=${admission().runtime_bound_player_sessions ?? "-"}`;
          }
        }), null);
        insert(_el$53, createComponent(Badge, {
          get children() {
            return `runtimeOnly=${admission().runtime_only_player_sessions ?? "-"}`;
          }
        }), null);
        insert(_el$53, createComponent(Badge, {
          get children() {
            return `runtimeProbe=${admission().runtime_probe_status || "-"}`;
          }
        }), null);
        insert(_el$53, createComponent(Badge, {
          get children() {
            return `issueBudget=${admission().remaining_issue_budget}`;
          }
        }), null);
        insert(_el$53, createComponent(Badge, {
          get children() {
            return `leaseTTL=${admission().slot_lease_ttl_ms}`;
          }
        }), null);
        insert(_el$53, createComponent(Badge, {
          get children() {
            return `issued=${admission().issued_players_total}`;
          }
        }), null);
        insert(_el$53, createComponent(Badge, {
          get children() {
            return `released=${admission().released_players_total}`;
          }
        }), null);
        return _el$53;
      })()
    }), _el$47);
    insert(_el$31, createComponent(Show, {
      get when() {
        return state$1.hostedAdmission?.runtime_probe_error;
      },
      get children() {
        var _el$39 = _tmpl$10();
        insert(_el$39, createComponent(Badge, {
          "class": "badge badge--warn",
          get children() {
            return `runtimeProbeError=${state$1.hostedAdmission.runtime_probe_error}`;
          }
        }));
        return _el$39;
      }
    }), _el$47);
    insert(_el$31, createComponent(Show, {
      get when() {
        return hostedRecoveryHint();
      },
      children: (hint) => (() => {
        var _el$54 = _tmpl$15(), _el$55 = _el$54.firstChild, _el$56 = _el$55.nextSibling, _el$57 = _el$56.firstChild, _el$58 = _el$57.nextSibling, _el$59 = _el$58.firstChild;
        insert(_el$57, createComponent(Badge, {
          "class": "badge badge--warn",
          get children() {
            return hint().kind;
          }
        }), null);
        insert(_el$57, createComponent(Badge, {
          get children() {
            return hint().title;
          }
        }), null);
        insert(_el$56, createComponent(EmptyState, {
          get children() {
            return hint().detail;
          }
        }), _el$58);
        _el$59.$$click = () => {
          void retryHostedPlayerIdentityIssue();
        };
        insert(_el$59, () => hint().cta);
        createRenderEffect(() => _el$59.disabled = state$1.auth.issueInFlight);
        return _el$54;
      })()
    }), _el$47);
    insert(_el$31, createComponent(Show, {
      get when() {
        return memo(() => !!(!state$1.auth.available && String(state$1.hostedAccess?.deployment_mode || "").trim() === "hosted_public_join"))() && !hostedRecoveryHint();
      },
      get children() {
        var _el$40 = _tmpl$11(), _el$41 = _el$40.firstChild;
        _el$41.$$click = () => {
          void retryHostedPlayerIdentityIssue();
        };
        createRenderEffect(() => _el$41.disabled = state$1.auth.issueInFlight);
        return _el$40;
      }
    }), _el$47);
    insert(_el$31, createComponent(Show, {
      get when() {
        return memo(() => !!state$1.auth.available)() && state$1.auth.source !== "legacy_viewer_auth_bootstrap";
      },
      get children() {
        var _el$42 = _tmpl$12(), _el$43 = _el$42.firstChild;
        _el$43.$$click = () => {
          void logoutHostedPlayerSession();
        };
        return _el$42;
      }
    }), _el$47);
    insert(_el$31, createComponent(PanelSection, {
      title: "Session Ladder",
      get children() {
        return [createComponent(EmptyState, {
          get children() {
            return authSurface().currentTierReason;
          }
        }), (() => {
          var _el$44 = _tmpl$13();
          insert(_el$44, createComponent(For, {
            get each() {
              return authSurface().tiers;
            },
            children: (tier) => createComponent(EventCard, {
              get title() {
                return tier.label;
              },
              get badge() {
                return tier.status;
              },
              get badgeClass() {
                return tierBadgeClass(tier.status);
              },
              get meta() {
                return tier.reason;
              }
            })
          }));
          return _el$44;
        })(), (() => {
          var _el$45 = _tmpl$10();
          insert(_el$45, createComponent(Badge, {
            get ["class"]() {
              return authSurface().capabilities.prompt_control.enabled ? "badge badge--good" : "badge badge--warn";
            },
            get children() {
              return `prompt=${authSurface().capabilities.prompt_control.enabled ? "enabled" : authSurface().capabilities.prompt_control.code}`;
            }
          }), null);
          insert(_el$45, createComponent(Badge, {
            get ["class"]() {
              return authSurface().capabilities.agent_chat.enabled ? "badge badge--good" : "badge badge--warn";
            },
            get children() {
              return `chat=${authSurface().capabilities.agent_chat.enabled ? "enabled" : authSurface().capabilities.agent_chat.code}`;
            }
          }), null);
          insert(_el$45, createComponent(Badge, {
            "class": "badge badge--warn",
            get children() {
              return `mainToken=${authSurface().capabilities.main_token_transfer.code}`;
            }
          }), null);
          return _el$45;
        })(), createComponent(EmptyState, {
          get children() {
            return authSurface().reconnect;
          }
        })];
      }
    }), _el$47);
    insert(_el$31, createComponent(Show, {
      get when() {
        return hostedActionMatrixView().length > 0;
      },
      get children() {
        return createComponent(PanelSection, {
          title: "Hosted Action Matrix",
          get children() {
            return [createComponent(EmptyState, {
              children: "This is the hosted public-join truth surface exported by the launcher. QA should read these action ids directly instead of inferring from button state alone."
            }), (() => {
              var _el$46 = _tmpl$13();
              insert(_el$46, createComponent(For, {
                get each() {
                  return hostedActionMatrixView();
                },
                children: (item) => createComponent(EventCard, {
                  get title() {
                    return item.actionId;
                  },
                  get badge() {
                    return memo(() => !!item.enabled)() ? "enabled" : item.code || "blocked";
                  },
                  get badgeClass() {
                    return item.enabled ? "badge badge--good" : "badge badge--warn";
                  },
                  get meta() {
                    return `required_auth=${item.requiredAuth} · availability=${item.availability}`;
                  },
                  get children() {
                    return [createComponent(EmptyState, {
                      get children() {
                        return item.reason || "-";
                      }
                    }), createComponent(Show, {
                      get when() {
                        return memo(() => !!item.capabilityReason)() && item.capabilityReason !== item.reason;
                      },
                      get children() {
                        return createComponent(EmptyState, {
                          get children() {
                            return `viewer=${item.capabilityReason}`;
                          }
                        });
                      }
                    })];
                  }
                })
              }));
              return _el$46;
            })()];
          }
        });
      }
    }), _el$47);
    insert(_el$31, createComponent(PlaybackControls, {
      get controlFeedback() {
        return controlFeedback();
      }
    }), _el$47);
    insert(_el$47, createComponent(MetricCard, {
      label: "Prompt Feedback",
      get value() {
        return promptFeedback()?.stage || "idle";
      },
      get children() {
        return createComponent(Show, {
          get when() {
            return promptFeedback();
          },
          get children() {
            return createComponent(Badge, {
              get ["class"]() {
                return feedbackBadgeClass(promptFeedback());
              },
              get children() {
                return promptFeedback()?.effect || promptFeedback()?.reason || "ready";
              }
            });
          }
        });
      }
    }), null);
    insert(_el$47, createComponent(MetricCard, {
      label: "Chat Feedback",
      get value() {
        return chatFeedback()?.stage || "idle";
      },
      get children() {
        return createComponent(Show, {
          get when() {
            return chatFeedback();
          },
          get children() {
            return createComponent(Badge, {
              get ["class"]() {
                return feedbackBadgeClass(chatFeedback());
              },
              get children() {
                return chatFeedback()?.effect || chatFeedback()?.reason || "ready";
              }
            });
          }
        });
      }
    }), null);
    insert(_el$50, createComponent(Show, {
      get when() {
        return state$1.recentEvents.length > 0;
      },
      get fallback() {
        return createComponent(EmptyState, {
          children: "Waiting for live events…"
        });
      },
      get children() {
        return createComponent(For, {
          get each() {
            return state$1.recentEvents;
          },
          children: (event) => createComponent(EventCard, {
            get title() {
              return summarizeEventTitle(event);
            },
            get badge() {
              return `#${Number(event.id || 0)}`;
            },
            get meta() {
              return `time=${Number(event.time || 0)}`;
            },
            get children() {
              return createComponent(JsonBlock, {
                get value() {
                  return event.kind;
                }
              });
            }
          })
        });
      }
    }));
    return _el$31;
  })();
}
function PlaybackControls(props) {
  const [stepCount, setStepCount] = createSignal(3);
  return createComponent(PanelSection, {
    title: "Playback Controls",
    get children() {
      return [(() => {
        var _el$60 = _tmpl$16(), _el$61 = _el$60.firstChild, _el$62 = _el$61.nextSibling, _el$63 = _el$62.nextSibling;
        _el$61.$$click = () => sendControl("play", null);
        _el$62.$$click = () => sendControl("pause", null);
        _el$63.$$click = () => sendControl("step", null);
        return _el$60;
      })(), (() => {
        var _el$64 = _tmpl$17(), _el$65 = _el$64.firstChild, _el$66 = _el$65.firstChild, _el$67 = _el$66.nextSibling, _el$68 = _el$65.nextSibling, _el$69 = _el$68.firstChild;
        _el$67.$$input = (event) => setStepCount(Math.max(1, Math.floor(Number(event.currentTarget.value || 1))));
        _el$69.$$click = () => sendControl("step", {
          count: Math.max(1, Math.floor(stepCount() || 1))
        });
        createRenderEffect(() => _el$67.value = stepCount());
        return _el$64;
      })(), createComponent(Show, {
        get when() {
          return props.controlFeedback;
        },
        get fallback() {
          return createComponent(EmptyState, {
            children: "No control feedback yet."
          });
        },
        children: (feedback) => [(() => {
          var _el$70 = _tmpl$10();
          insert(_el$70, createComponent(Badge, {
            get children() {
              return `action=${feedback().action}`;
            }
          }), null);
          insert(_el$70, createComponent(Badge, {
            get children() {
              return `stage=${feedback().stage}`;
            }
          }), null);
          insert(_el$70, createComponent(Badge, {
            get children() {
              return `Δtick=${feedback().deltaLogicalTime}`;
            }
          }), null);
          insert(_el$70, createComponent(Badge, {
            get children() {
              return `Δevent=${feedback().deltaEventSeq}`;
            }
          }), null);
          return _el$70;
        })(), createComponent(JsonBlock, {
          get value() {
            return feedback();
          }
        })]
      })];
    }
  });
}
function InteractionPanel() {
  const agentId = () => selectedAgentId();
  const authSurface = () => buildAuthSurfaceModel();
  const promptCapability = () => authSurface().capabilities.prompt_control;
  const chatCapability = () => authSurface().capabilities.agent_chat;
  const mainTokenTransferCapability = () => authSurface().capabilities.main_token_transfer;
  const mainTokenTransferPolicy = () => hostedActionPolicy("main_token_transfer");
  const binding = () => selectedAgentBindingInfo();
  const debugContext = () => selectedAgentExecutionDebugContext();
  const promptFeedback = () => snapshotSemanticFeedback(state.lastPromptFeedback);
  const chatFeedback = () => snapshotSemanticFeedback(state.lastChatFeedback);
  const chatHistory = () => state.chatHistory.filter((entry) => entry.agentId === agentId() || entry.targetAgentId === agentId()).slice(0, 12);
  const interactionEnabled = () => promptCapability().enabled;
  const assetLaneStatusText = () => mainTokenTransferCapability().enabled ? "preview_only" : mainTokenTransferCapability().code || "blocked";
  const assetLaneDetail = () => mainTokenTransferCapability().enabled ? "Contract marks main_token_transfer as strong_auth-capable on this lane, but software_safe still exposes no transfer form here." : mainTokenTransferCapability().reason;
  if (!agentId()) {
    return createComponent(EmptyState, {
      children: "Select an agent to unlock prompt/chat controls."
    });
  }
  return (() => {
    var _el$71 = _tmpl$28(), _el$72 = _el$71.firstChild, _el$74 = _el$72.nextSibling;
    insert(_el$72, createComponent(Badge, {
      "class": "badge badge--accent",
      children: "Agent Interaction"
    }), null);
    insert(_el$72, createComponent(Badge, {
      get children() {
        return `agent=${agentId()}`;
      }
    }), null);
    insert(_el$72, createComponent(Badge, {
      get children() {
        return `promptVersion=${Number(state.promptDraft.currentVersion || 0)}`;
      }
    }), null);
    insert(_el$71, createComponent(Show, {
      get when() {
        return debugContext()?.provider_mode === "openclaw_local_http";
      },
      get children() {
        return createComponent(EmptyState, {
          get children() {
            return `Selected agent currently runs through OpenClaw(Local HTTP) in ${debugContext()?.execution_mode || "headless_agent"}; software_safe stays in debug_viewer observer-only mode, so prompt/chat are intentionally disabled here.`;
          }
        });
      }
    }), _el$74);
    insert(_el$71, createComponent(Show, {
      get when() {
        return debugContext()?.provider_mode !== "openclaw_local_http";
      },
      get children() {
        return createComponent(Show, {
          get when() {
            return interactionEnabled();
          },
          get fallback() {
            return createComponent(EmptyState, {
              get children() {
                return promptCapability().reason;
              }
            });
          },
          get children() {
            return [(() => {
              var _el$73 = _tmpl$10();
              insert(_el$73, createComponent(Badge, {
                "class": "badge badge--good",
                get children() {
                  return authSurface().currentTier;
                }
              }), null);
              insert(_el$73, createComponent(Badge, {
                get children() {
                  return `player=${state.auth.playerId}`;
                }
              }), null);
              insert(_el$73, createComponent(Badge, {
                get children() {
                  return `source=${authSurface().source}`;
                }
              }), null);
              return _el$73;
            })(), createComponent(EmptyState, {
              get children() {
                return promptCapability().reason;
              }
            })];
          }
        });
      }
    }), _el$74);
    insert(_el$74, createComponent(Badge, {
      get children() {
        return `boundPlayer=${binding()?.playerId || "-"}`;
      }
    }), null);
    insert(_el$74, createComponent(Badge, {
      get children() {
        return `boundKey=${binding()?.publicKey ? `${binding().publicKey.slice(0, 10)}…` : "-"}`;
      }
    }), null);
    insert(_el$74, createComponent(Badge, {
      get ["class"]() {
        return promptCapability().enabled ? "badge badge--good" : "badge badge--warn";
      },
      get children() {
        return `prompt=${promptCapability().enabled ? "enabled" : promptCapability().code}`;
      }
    }), null);
    insert(_el$74, createComponent(Badge, {
      get ["class"]() {
        return chatCapability().enabled ? "badge badge--good" : "badge badge--warn";
      },
      get children() {
        return `chat=${chatCapability().enabled ? "enabled" : chatCapability().code}`;
      }
    }), null);
    insert(_el$74, createComponent(Badge, {
      get ["class"]() {
        return mainTokenTransferCapability().enabled ? "badge badge--good" : "badge badge--warn";
      },
      get children() {
        return `mainToken=${assetLaneStatusText()}`;
      }
    }), null);
    insert(_el$71, createComponent(EmptyState, {
      get children() {
        return assetLaneDetail();
      }
    }), null);
    insert(_el$71, createComponent(PanelSection, {
      title: "Prompt Overrides",
      get children() {
        return [createComponent(Show, {
          get when() {
            return memo(() => !!authSurface().capabilities.prompt_control.enabled)() && String(state.hostedAccess?.deployment_mode || "").trim() === "hosted_public_join";
          },
          get children() {
            var _el$75 = _tmpl$18(), _el$76 = _el$75.firstChild, _el$77 = _el$76.nextSibling;
            _el$77.$$input = (event) => {
              state.strongAuth.approvalCode = String(event.currentTarget.value || "");
            };
            createRenderEffect(() => _el$77.value = state.strongAuth.approvalCode || "");
            return _el$75;
          }
        }), (() => {
          var _el$78 = _tmpl$19(), _el$79 = _el$78.firstChild, _el$80 = _el$79.nextSibling;
          _el$80.$$input = (event) => {
            state.promptDraft.systemPrompt = String(event.currentTarget.value || "");
            state.promptDraft.dirty = true;
          };
          createRenderEffect(() => _el$80.disabled = !promptCapability().enabled);
          createRenderEffect(() => _el$80.value = state.promptDraft.systemPrompt);
          return _el$78;
        })(), (() => {
          var _el$81 = _tmpl$20(), _el$82 = _el$81.firstChild, _el$83 = _el$82.nextSibling;
          _el$83.$$input = (event) => {
            state.promptDraft.shortTermGoal = String(event.currentTarget.value || "");
            state.promptDraft.dirty = true;
          };
          createRenderEffect(() => _el$83.disabled = !promptCapability().enabled);
          createRenderEffect(() => _el$83.value = state.promptDraft.shortTermGoal);
          return _el$81;
        })(), (() => {
          var _el$84 = _tmpl$21(), _el$85 = _el$84.firstChild, _el$86 = _el$85.nextSibling;
          _el$86.$$input = (event) => {
            state.promptDraft.longTermGoal = String(event.currentTarget.value || "");
            state.promptDraft.dirty = true;
          };
          createRenderEffect(() => _el$86.disabled = !promptCapability().enabled);
          createRenderEffect(() => _el$86.value = state.promptDraft.longTermGoal);
          return _el$84;
        })(), (() => {
          var _el$87 = _tmpl$22(), _el$88 = _el$87.firstChild, _el$89 = _el$88.nextSibling;
          _el$88.$$click = () => sendPromptControl("preview", null);
          _el$89.$$click = () => sendPromptControl("apply", null);
          createRenderEffect((_p$) => {
            var _v$5 = !promptCapability().enabled, _v$6 = !promptCapability().enabled;
            _v$5 !== _p$.e && (_el$88.disabled = _p$.e = _v$5);
            _v$6 !== _p$.t && (_el$89.disabled = _p$.t = _v$6);
            return _p$;
          }, {
            e: void 0,
            t: void 0
          });
          return _el$87;
        })(), (() => {
          var _el$90 = _tmpl$23(), _el$91 = _el$90.firstChild, _el$92 = _el$91.firstChild, _el$93 = _el$92.nextSibling, _el$94 = _el$91.nextSibling;
          _el$93.$$input = (event) => {
            const nextValue = Number(event.currentTarget.value || 0);
            state.promptDraft.rollbackTargetVersion = Math.max(0, Math.floor(nextValue || 0));
          };
          _el$94.$$click = () => {
            sendPromptControl("rollback", {
              toVersion: Number(state.promptDraft.rollbackTargetVersion || 0)
            });
          };
          createRenderEffect((_p$) => {
            var _v$7 = !promptCapability().enabled, _v$8 = !promptCapability().enabled;
            _v$7 !== _p$.e && (_el$93.disabled = _p$.e = _v$7);
            _v$8 !== _p$.t && (_el$94.disabled = _p$.t = _v$8);
            return _p$;
          }, {
            e: void 0,
            t: void 0
          });
          createRenderEffect(() => _el$93.value = Number(state.promptDraft.rollbackTargetVersion || 0));
          return _el$90;
        })(), createComponent(Show, {
          get when() {
            return promptFeedback();
          },
          get fallback() {
            return createComponent(EmptyState, {
              children: "No prompt feedback yet."
            });
          },
          children: (feedback) => [(() => {
            var _el$105 = _tmpl$10();
            insert(_el$105, createComponent(Badge, {
              get ["class"]() {
                return feedbackBadgeClass(feedback());
              },
              get children() {
                return feedback().stage;
              }
            }));
            return _el$105;
          })(), createComponent(JsonBlock, {
            get value() {
              return feedback();
            }
          })]
        }), createComponent(Show, {
          get when() {
            return state.strongAuth.lastGrantActionId;
          },
          get children() {
            return createComponent(EmptyState, {
              get children() {
                return `lastGrant=${state.strongAuth.lastGrantActionId} expiresAt=${state.strongAuth.lastGrantExpiresAtUnixMs || "-"}`;
              }
            });
          }
        }), createComponent(Show, {
          get when() {
            return state.strongAuth.lastGrantError;
          },
          get children() {
            return createComponent(EmptyState, {
              style: "color:var(--bad);",
              get children() {
                return state.strongAuth.lastGrantError;
              }
            });
          }
        })];
      }
    }), null);
    insert(_el$71, createComponent(PanelSection, {
      title: "Asset / Governance Lane",
      get children() {
        return [(() => {
          var _el$95 = _tmpl$10();
          insert(_el$95, createComponent(Badge, {
            get ["class"]() {
              return mainTokenTransferCapability().enabled ? "badge badge--good" : "badge badge--warn";
            },
            get children() {
              return `main_token_transfer=${assetLaneStatusText()}`;
            }
          }), null);
          insert(_el$95, createComponent(Badge, {
            get children() {
              return `required_auth=${mainTokenTransferPolicy()?.required_auth || "-"}`;
            }
          }), null);
          insert(_el$95, createComponent(Badge, {
            get children() {
              return `availability=${mainTokenTransferPolicy()?.availability || "-"}`;
            }
          }), null);
          return _el$95;
        })(), createComponent(EmptyState, {
          get children() {
            return assetLaneDetail();
          }
        }), createComponent(EmptyState, {
          get children() {
            return mainTokenTransferPolicy()?.reason || "No hosted action policy is available for main_token_transfer on this lane.";
          }
        }), _tmpl$24()];
      }
    }), null);
    insert(_el$71, createComponent(PanelSection, {
      title: "Agent Chat",
      get children() {
        return [(() => {
          var _el$97 = _tmpl$25(), _el$98 = _el$97.firstChild, _el$99 = _el$98.nextSibling;
          _el$99.$$input = (event) => {
            state.chatDraft.message = String(event.currentTarget.value || "");
            state.chatDraft.dirty = true;
          };
          createRenderEffect(() => _el$99.disabled = !chatCapability().enabled);
          createRenderEffect(() => _el$99.value = state.chatDraft.message);
          return _el$97;
        })(), (() => {
          var _el$100 = _tmpl$26(), _el$101 = _el$100.firstChild;
          _el$101.$$click = () => sendAgentChat(agentId(), state.chatDraft.message);
          createRenderEffect(() => _el$101.disabled = !chatCapability().enabled);
          return _el$100;
        })(), createComponent(Show, {
          get when() {
            return chatFeedback();
          },
          get fallback() {
            return createComponent(EmptyState, {
              children: "No chat feedback yet."
            });
          },
          children: (feedback) => [(() => {
            var _el$106 = _tmpl$10();
            insert(_el$106, createComponent(Badge, {
              get ["class"]() {
                return feedbackBadgeClass(feedback());
              },
              get children() {
                return feedback().stage;
              }
            }));
            return _el$106;
          })(), createComponent(JsonBlock, {
            get value() {
              return feedback();
            }
          })]
        }), (() => {
          var _el$102 = _tmpl$27(), _el$103 = _el$102.firstChild, _el$104 = _el$103.nextSibling;
          insert(_el$104, createComponent(Show, {
            get when() {
              return chatHistory().length > 0;
            },
            get fallback() {
              return createComponent(EmptyState, {
                children: "No chat history for this agent yet."
              });
            },
            get children() {
              return createComponent(For, {
                get each() {
                  return chatHistory();
                },
                children: (entry) => createComponent(EventCard, {
                  get title() {
                    return memo(() => entry.source === "player")() ? `player → ${entry.targetAgentId || entry.agentId || "agent"}` : `${entry.agentId || "agent"} spoke`;
                  },
                  get badge() {
                    return `tick=${Number(entry.tick || 0)}`;
                  },
                  get meta() {
                    return `speaker=${entry.speaker || entry.playerId || "-"} · location=${entry.locationId || "-"}`;
                  },
                  get children() {
                    return createComponent(JsonBlock, {
                      value: entry
                    });
                  }
                })
              });
            }
          }));
          return _el$102;
        })()];
      }
    }), null);
    return _el$71;
  })();
}
function DetailsPanel() {
  const selectedLabel = () => state.selectedKind && state.selectedId ? `${state.selectedKind}:${state.selectedId}` : "nothing selected";
  const snapshotSummary = () => ({
    config: state.snapshot?.config || null,
    counts: {
      agents: Object.keys(state.snapshot?.model?.agents || {}).length,
      locations: Object.keys(state.snapshot?.model?.locations || {}).length,
      promptProfiles: Object.keys(state.snapshot?.model?.agent_prompt_profiles || {}).length,
      executionDebugContexts: Object.keys(state.snapshot?.model?.agent_execution_debug_contexts || {}).length
    },
    metrics: state.metrics,
    hostedAccess: clone(state.hostedAccess)
  });
  return (() => {
    var _el$107 = _tmpl$30(), _el$108 = _el$107.firstChild, _el$109 = _el$108.nextSibling;
    _el$109.firstChild;
    insert(_el$108, createComponent(Badge, {
      "class": "badge badge--accent",
      children: "Selected"
    }), null);
    insert(_el$108, createComponent(Badge, {
      get children() {
        return selectedLabel();
      }
    }), null);
    insert(_el$107, createComponent(InteractionPanel, {}), _el$109);
    insert(_el$107, createComponent(Show, {
      get when() {
        return state.selectedObject;
      },
      get fallback() {
        return createComponent(EmptyState, {
          children: "Select an agent or location from the left list."
        });
      },
      get children() {
        return createComponent(JsonBlock, {
          get value() {
            return clone(state.selectedObject);
          }
        });
      }
    }), _el$109);
    insert(_el$109, createComponent(JsonBlock, {
      get value() {
        return snapshotSummary();
      }
    }), null);
    insert(_el$107, createComponent(Show, {
      get when() {
        return state.lastError;
      },
      get children() {
        var _el$111 = _tmpl$29(), _el$112 = _el$111.firstChild, _el$113 = _el$112.nextSibling;
        insert(_el$113, () => state.lastError);
        return _el$111;
      }
    }), null);
    return _el$107;
  })();
}
function AppShell() {
  return [(() => {
    var _el$114 = _tmpl$31(), _el$115 = _el$114.firstChild, _el$116 = _el$115.nextSibling;
    insert(_el$116, createComponent(TargetsPanel, {}));
    return _el$114;
  })(), (() => {
    var _el$117 = _tmpl$32(), _el$118 = _el$117.firstChild, _el$119 = _el$118.nextSibling;
    insert(_el$119, createComponent(WorldSummaryPanel, {}));
    return _el$117;
  })(), (() => {
    var _el$120 = _tmpl$33(), _el$121 = _el$120.firstChild, _el$122 = _el$121.nextSibling;
    insert(_el$122, createComponent(DetailsPanel, {}));
    return _el$120;
  })()];
}
const app = document.getElementById("app");
if (!app) {
  throw new Error("software_safe root #app is missing");
}
let dispose = render$1(() => createComponent(AppShell, {}), app);
setRenderHook(() => {
  dispose();
  app.textContent = "";
  dispose = render$1(() => createComponent(AppShell, {}), app);
});
initializeSoftwareSafeCore();
delegateEvents(["input", "click"]);
