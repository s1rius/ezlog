<template>
  <div class="log-container">
    <div class="tab flex justify-between items-center bg-gray-200 p-1 sticky top-0 z-10">
      <span class="file-name pointer-events-none ml-3">{{ fileName }}</span>
      <button class="close-btn ml-auto" @click="closeFile">x</button>
    </div>
    <div class="log-entry ml-4 mr-4" v-for="(log, index) in logs" :key="index">
      <div class="log-item" :class="{
            'level-trace': log.level === 'Trace',
            'level-debug': log.level === 'Debug',
            'level-info': log.level === 'Info',
            'level-warn': log.level === 'Warn',
            'level-error': log.level === 'Error',
          }">
        <span class="log-time">{{ log.time }}</span>
        <span
          class="log-level">
          {{ log.level }}
        </span>
        <span class="log-target">{{ log.target }}</span>
        
        <span  class="log-content">{{
          log.content
        }}</span>
      </div>
    </div>
  </div>
</template>

<script>
export default {
  props: {
    logs: {
      type: Array,
      required: true,
    },
    filePath: {
      type: String,
      required: true,
    },
    onClose: {
      type: Function,
      required: true,
    },
  },
  computed: {
    fileName() {
      return this.filePath.split('/').pop();
    },
    dirPath() {
      return this.filePath.split('/').slice(0, -1).join('/');
    },
  },
    methods: {
        closeFile() {
            this.onClose();
        },

        getLevelColor(level) {
            switch (level) {
                case 'Trace':
                    return 'rgb(156, 163, 175)';
                case 'Debug':
                    return 'rgb(33, 150, 243)';
                case 'Info':
                    return 'rgb(76, 175, 80)';
                case 'Warn':
                    return 'rgb(255, 193, 7)';
                case 'Error':
                    return 'rgb(244, 67, 54)';
                default:
                    return 'black';
            }
        },
    },
};
</script>

<style scoped>
.log-container {
    font-family:  'Lucida Console', 'Andale Mono', 'Monaco', 'Consolas', 'Source Code Pro', 'Fira Code', 'Roboto Mono', 'Ubuntu Mono', 'Inconsolata', monospace;
}

.log-entry {
  padding: 2px 0;
}

.tab {
  display: flex;
  justify-content: space-between;
  align-items: center;
  background-color: #25272b97;
}

.file-name {
  pointer-events: none; /* Make non-clickable */
  user-select: none; /* Prevent text selection */
}

.log-item {
   text-align: start;
}

.log-target {
  min-width: 80px;
  white-space: nowrap;
  font-family:  Courier;
}

.close-btn {
  background: transparent;
  border: none;
  width: 28px;
  height: 28px;
  font-size: 18px;
  cursor: pointer;
  margin-right: 12px;
  transition: background-color 0.3s;
  outline: none;
  box-shadow: none;
}

.close-btn:hover {
  background-color: rgba(0, 0, 0, 0.1); /* Dark background on hover */
}

.log-time {
    display: inline-block;  
  width: 28ch;
  white-space: nowrap;
}

.log-level {
    display: inline-block; 
    width: 6ch; 
  font-weight: bold;
  text-align: start;
  align-items: baseline;
  white-space: nowrap;
}

.level-trace {
    color: rgb(156, 163, 175);
}

.level-debug {
  color: rgb(33, 150, 243);
}

.level-info {
  color: rgb(76, 175, 80);
}

.level-warn {
  color: rgb(255, 193, 7);
}

.level-error {
  color: rgb(244, 67, 54);
}

.log-content {
  margin-left: 10px;
  white-space: pre-wrap;
  word-break: break-all;
}
</style>
