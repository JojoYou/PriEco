document.addEventListener('DOMContentLoaded', function () {
  answer();
  peopleAsk();
});

function answer() {
  const quickAnswerElement = document.querySelector('.quickAnswer');
  if (!quickAnswerElement) {
    return;
  }
  const data = quickAnswerElement.querySelector('#answerData').innerText;
  const que = quickAnswerElement.querySelector('#answerQuestion').innerText;

  const answerAnswerElement = quickAnswerElement.querySelector('#answerAnswer');

  const dataToSend = {
    textData: data,
    question: que
  };  
   fetch('api/answer.php', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json'
    },
    body: JSON.stringify({dataToSend})
    })
    .then(response => response.json())
    .then(returnData => {
      if (answerAnswerElement && returnData['score'] >= 0.3) {
        if (answerLoadingElement) {
          answerLoadingElement.remove();
      }

        const elementsWithClass = quickAnswerElement.querySelectorAll('.answerHide');
        elementsWithClass.forEach(element => {
          element.classList.remove('answerHide');
        });

        let sentences = data.split('.');
        let targetSentence = sentences.find(sentence => sentence.includes(returnData['answer']));
        if (targetSentence) {
          const maxLength = 150;

          const index = targetSentence.indexOf(returnData['answer']);
          
          const start = Math.max(0, index - maxLength);
          
          const end = Math.min(targetSentence.length, index + returnData['answer'].length + maxLength);
          
          let truncatedSentence = targetSentence.substring(start, end);
          truncatedSentence = (start > 0) ? '...' + truncatedSentence : truncatedSentence;
          truncatedSentence = (end < targetSentence.length) ? truncatedSentence + '...' : truncatedSentence;
          
          const pattern = new RegExp(returnData['answer'], 'g');
          const modifiedSentence = truncatedSentence.replace(pattern, `<b>${returnData['answer']}</b>`);
          
          answerAnswerElement.innerHTML = modifiedSentence;
        }
      }
      else{
        quickAnswerElement.classList.add('none');
      }
    })
    .catch(error => {
      answerLoadingElement.remove();
    });
    const answerLoadingElement = quickAnswerElement.querySelector('#answerLoading');

}

function peopleAsk(){
  const peopleAskElement = document.querySelector('.peopleAskFor');
  if (!peopleAskElement) {
    return;
  }
  const dataValue = peopleAskElement.getAttribute('data');

  const dataToSend = {
    url: dataValue
  };  
  let c = 0;

  fetch('api/peopleAsk.php', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json'
    },
    body: JSON.stringify({dataToSend})
    })
    .then(response => response.json())
    .then(returnData => {
      returnData.forEach(element => {
        const question = peopleAskElement.querySelector('#peopleAskQuestion'+c);
        const parentElement = question.parentNode;
        const loadingDots = parentElement.querySelector('.lds-ellipsis');
        loadingDots.remove();

        question.innerHTML = element.question + "<img src='View/icon/dropdown.svg' class='peopleAskIcon ml-10 filterImage float-right width20'>";

        const answer = peopleAskElement.querySelector('.peopleAskP'+c);
        answer.innerHTML = element.answer;

        
        c++;
      });

          while(c <= 4){
          const question = peopleAskElement.querySelector('#peopleAskQuestion'+c);

          const parentElement = question.parentNode;
          parentElement.classList.add('none');
          c++;
        }

      if(c == 0){
        peopleAskElement.remove();
      }
    })
    .catch(error => {
    });
}