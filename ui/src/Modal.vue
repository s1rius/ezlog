<script setup>
import { ref } from "vue";

const props = defineProps({
    show: Boolean
})

const key = ref('')
const nonce = ref('')

</script>

<template>
    <Transition name="modal">
        <div v-if="show" class="modal-mask">
            <div class="modal-container bg-white dark:bg-stone-700">
                <div class="modal-header">
                    <slot name="header">default header</slot>
                </div>

                <div class="modal-body">
                    <slot name="body">
                        <div>
                            <input class="px-5 py-2" v-model="key" placeholder="enter key" />
                            <input class="px-5 py-2 mt-4" v-model="nonce" placeholder="enter nonce"/>
                        </div>
                    </slot>
                </div>

                <div class="modal-footer">
                    <slot name="footer">
                        <button class="modal-default-button px-5 py-2" @click="$emit('submit', key, nonce)">OK</button>
                    </slot>
                </div>
            </div>
        </div>
    </Transition>
</template>

<style>
.modal-mask {
    position: fixed;
    z-index: 9998;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    background-color: rgba(0, 0, 0, 0.5);
    display: flex;
    transition: opacity 0.3s ease;
}

.modal-container {
    width: 300px;
    margin: auto;
    padding: 20px 30px;
    border-radius: 2px;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.33);
    transition: all 0.3s ease;
}

.modal-header h3 {
    margin-top: 0;
    color: #42b983;
}

.modal-body {
    margin: 20px 0;
}

.modal-default-button {
    float: right;
}

/*
 * The following styles are auto-applied to elements with
 * transition="modal" when their visibility is toggled
 * by Vue.js.
 *
 * You can easily play with the modal transition by editing
 * these styles.
 */

.modal-enter-from {
    opacity: 0;
}

.modal-leave-to {
    opacity: 0;
}

.modal-enter-from .modal-container,
.modal-leave-to .modal-container {
    -webkit-transform: scale(1.1);
    transform: scale(1.1);
}
</style>