function b() { this.c = () => 0; }
enqueueMark(new b());
setMarkStackLimit(16);
gc();
