pipeline {
  agent {
    docker {
      image 'python:3.9-bullseye'
    }

  }
  stages {
    stage('Build Python Package') {
      steps {
        sh '''cd src/python

/usr/local/bin/python -m pip install --upgrade pip

pip install -e .

cd

twine upload src/python/dist/* --non-interactive

'''
      }
    }

  }
}