package wtf.s1.buildsrc

object Versions {
    const val kotlin = "1.5.30"
    const val ktx = "1.0.0"
    const val coroutines = "1.5.2"
    const val gradlePlugin = "7.1.1"
    const val lifecycle = "2.2.0"
    const val minSdkVersion = 21
    const val targetSdkVersion = 31
    const val versionCode = 1
    const val libVersion = "0.1.1"
}

object Plugins {
    const val androidLib = "com.android.library"
}

object Deps {

    object Kotlin {
        const val stdLib = "org.jetbrains.kotlin:kotlin-stdlib-jdk8:${Versions.kotlin}"
        const val coroutines =
            "org.jetbrains.kotlinx:kotlinx-coroutines-core:${Versions.coroutines}"
        const val coroutinesAndroid =
            "org.jetbrains.kotlinx:kotlinx-coroutines-android:${Versions.coroutines}"
        const val ktxCore = "androidx.core:core-ktx:${Versions.ktx}"
    }

    object AndroidX {
        const val appcompat = "androidx.appcompat:appcompat:1.2.0"
        const val constraintLayout = "androidx.constraintlayout:constraintlayout:1.1.3"
        const val recyclerview = "androidx.recyclerview:recyclerview:1.1.0"
        const val extension = "androidx.lifecycle:lifecycle-extensions:${Versions.lifecycle}"
        const val livedata = "androidx.lifecycle:lifecycle-livedata:${Versions.lifecycle}"
        const val lifecycleRuntime =
            "androidx.lifecycle:lifecycle-runtime-ktx:${Versions.lifecycle}"
        const val annotation = "androidx.annotation:annotation:1.3.0"

        object Core {
            const val utils = "androidx.legacy:legacy-support-core-utils:1.0.0"
        }
    }
}

object ClassPaths {
    const val gradlePlugin = "com.android.tools.build:gradle:${Versions.gradlePlugin}"
    const val gradleApi = "com.android.tools.build:gradle-api:${Versions.gradlePlugin}"
    const val kotlinPlugin = "org.jetbrains.kotlin:kotlin-gradle-plugin:${Versions.kotlin}"
    const val vanniktechMavenPublishPlugin = "com.vanniktech:gradle-maven-publish-plugin:0.18.0"
    const val dokaa = "org.jetbrains.dokka:dokka-gradle-plugin:1.6.10"
}