package wtf.s1.buildsrc

object Versions {
    const val kotlin = "1.5.30"
    const val ktx = "1.0.0"
    const val coroutines = "1.5.2"
    const val gradlePlugin = "7.1.2"
    const val lifecycle = "2.2.0"
    const val minSdkVersion = 21
    const val targetSdkVersion = 33
    const val compileVersion = 33
    const val versionCode = 1
    const val libVersion = "0.1.7"
    const val benchmarkVersion = "1.1.0"
    const val androidxTestVersion = "1.4.0"
    const val espressoCoreVersion = "3.4.0"
    const val jUnitVersion = "4.13.2"
    const val testExtVersion = "1.1.3"
    const val macro = "1.1.1"
}

object Plugins {
    const val androidLib = "com.android.library"
}

object Deps {

    object Kotlin {
        const val stdkt = "org.jetbrains.kotlin:kotlin-stdlib:${Versions.kotlin}"
        const val stdLib_jdk8 = "org.jetbrains.kotlin:kotlin-stdlib-jdk8:${Versions.kotlin}"
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

        const val benchmarkJunit4 =
            "androidx.benchmark:benchmark-junit4:${Versions.benchmarkVersion}"
        const val benchmarkMacro = "androidx.benchmark:benchmark-macro-junit4:${Versions.macro}"

        const val testJunit = "androidx.test.ext:junit:${Versions.testExtVersion}"
        const val rules = "androidx.test:rules:${Versions.androidxTestVersion}"
        const val test = "androidx.test:runner:${Versions.androidxTestVersion}"
        const val junit = "junit:junit:${Versions.jUnitVersion}"

        const val espresso =  "androidx.test.espresso:espresso-core:${Versions.espressoCoreVersion}"
    }

    val xlog = "com.tencent.mars:mars-xlog:1.2.5"
    val logan = "com.dianping.android.sdk:logan:1.2.4"
}

object ClassPaths {
    const val gradlePlugin = "com.android.tools.build:gradle:${Versions.gradlePlugin}"
    const val gradleApi = "com.android.tools.build:gradle-api:${Versions.gradlePlugin}"
    const val kotlinPlugin = "org.jetbrains.kotlin:kotlin-gradle-plugin:${Versions.kotlin}"
    const val vanniktechMavenPublishPlugin = "com.vanniktech:gradle-maven-publish-plugin:0.21.0"
    const val dokaa = "org.jetbrains.dokka:dokka-gradle-plugin:1.6.10"
    const val benchmarkPlugin = "androidx.benchmark:benchmark-gradle-plugin:1.1.1"
}